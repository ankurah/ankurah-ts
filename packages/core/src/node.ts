// MIRRORS: ankurah/core/src/node.rs

import {
  type CollectionId,
  type EntityId,
  EntityId as EntityIdClass,
  type TransactionId,
  type Attested,
  type Event,
  Clock,
  EntityState,
} from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';

import { Entity, WeakEntitySet } from './entity.ts';
import { Context, type TContext } from './context.ts';
import type { Transaction } from './transaction.ts';
import { MutationError, RetrievalError } from './error.ts';
import type { AccessDenied } from './error.ts';
import type { EntityChange } from './changes.ts';
import type { StorageEngine, StorageCollection } from './storage.ts';
import type { PolicyAgent } from './policy.ts';

// ---------------------------------------------------------------------------
// MatchArgs — query parameters
// ---------------------------------------------------------------------------

/**
 * Query parameters for entity fetching.
 *
 * Rust: `pub struct MatchArgs { pub selection: Selection, pub cached: bool }`
 */
export interface MatchArgs {
  selection: Selection;
  cached: boolean;
}

/**
 * Create MatchArgs from a selection string or Selection object.
 */
export function matchArgs(selection: Selection, cached = true): MatchArgs {
  return { selection, cached };
}

// ---------------------------------------------------------------------------
// Node — the main participant in the ankurah network
// ---------------------------------------------------------------------------

/**
 * A participant in the Ankurah network, and primary place where queries are initiated.
 *
 * Rust: `pub struct Node<SE, PA>(Arc<NodeInner<SE, PA>>)`
 *
 * Divergence: Rust uses `Arc<NodeInner>` with Deref; TS uses a plain class [E8].
 * Divergence: Rust is generic over StorageEngine and PolicyAgent; TS uses interface
 *   fields rather than generics (simpler for JS runtime) [A6].
 * Divergence: Many Node methods that depend on networking/peers are deferred to
 *   later layers. This implementation provides the core entity management and
 *   context creation functionality.
 */
export class Node {
  /** Node identity */
  readonly id: EntityId;

  /** Whether this node has durable storage */
  readonly durable: boolean;

  /** Weak entity set for deduplication */
  readonly entities: WeakEntitySet;

  /** Storage engine for persistence */
  readonly storageEngine: StorageEngine;

  /** Policy agent for access control */
  readonly policyAgent: PolicyAgent<unknown>;

  /** Context data factory — creates context data for new contexts */
  private readonly defaultContextData: unknown;

  constructor(options: {
    id?: EntityId;
    durable?: boolean;
    storageEngine: StorageEngine;
    policyAgent: PolicyAgent<unknown>;
    contextData?: unknown;
  }) {
    this.id = options.id ?? EntityIdClass.new();
    this.durable = options.durable ?? false;
    this.entities = new WeakEntitySet();
    this.storageEngine = options.storageEngine;
    this.policyAgent = options.policyAgent;
    this.defaultContextData = options.contextData ?? null;
  }

  /**
   * Create a context with the given context data.
   *
   * Rust: `Context::new(node, data)`
   */
  context(contextData?: unknown): Context {
    const cdata = contextData ?? this.defaultContextData;
    const nodeContext = new NodeAndContext(this, cdata);
    return new Context(nodeContext);
  }

  /**
   * Fetch entities from local storage for a collection + selection.
   *
   * Rust: `pub(crate) async fn fetch_entities_from_local(...) -> Result<Vec<Entity>, RetrievalError>`
   */
  async fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Entity[]> {
    const collection = await this.storageEngine.collection(collectionId);
    const states = await collection.fetchStates(selection);
    const entities: Entity[] = [];
    for (const attestedState of states) {
      const [_changed, entity] = this.entities.withState(
        attestedState.payload.entityId,
        attestedState.payload.collection,
        attestedState.payload.state,
      );
      entities.push(entity);
    }
    return entities;
  }

  toString(): string {
    return `Node(${this.id.toBase64Short()})`;
  }
}

// ---------------------------------------------------------------------------
// NodeAndContext — concrete TContext implementation
// ---------------------------------------------------------------------------

/**
 * Concrete implementation of TContext that combines a Node with context data.
 *
 * Rust: `pub struct NodeAndContext<SE, PA: PolicyAgent> { pub node: Node<SE, PA>, pub cdata: PA::ContextData }`
 *
 * This is the bridge between Transaction (which uses TContext interface) and
 * Node (which owns the actual entities, storage, and reactor).
 */
export class NodeAndContext implements TContext {
  readonly node: Node;
  readonly cdata: unknown;

  constructor(node: Node, cdata: unknown) {
    this.node = node;
    this.cdata = cdata;
  }

  // ── TContext interface ──────────────────────────────────────────────

  nodeId(): EntityId {
    return this.node.id;
  }

  /**
   * Create a new entity for a transaction.
   *
   * Rust: creates primary entity in WeakEntitySet, then snapshots for transaction.
   */
  createEntity(collection: CollectionId, trxAlive: { value: boolean }): Entity {
    const primaryEntity = this.node.entities.create(collection);
    return primaryEntity.snapshot(trxAlive);
  }

  /**
   * Check write permissions.
   *
   * Rust: `self.node.policy_agent.check_write(&self.cdata, entity, None)`
   */
  checkWrite(entity: Entity): void {
    this.node.policyAgent.checkWrite(this.cdata, entity, null);
  }

  /**
   * Retrieve a single entity by ID.
   *
   * Simplified vs Rust: no peer fetching, just local storage lookup.
   * Full peer-assisted retrieval will be added when connectors are ported.
   */
  async getEntity(id: EntityId, collection: CollectionId, cached: boolean): Promise<Entity> {
    // Check local resident entities first
    const local = this.node.entities.get(id);
    if (local) {
      return local;
    }

    // Fetch from storage
    const storageCollection = await this.node.storageEngine.collection(collection);
    try {
      const entityState = await storageCollection.getState(id);
      const [_changed, entity] = this.node.entities.withState(
        id,
        collection,
        entityState.payload.state,
      );
      return entity;
    } catch (e) {
      throw RetrievalError.entityNotFound(id);
    }
  }

  /**
   * Get a resident entity from the local weak entity set.
   */
  getResidentEntity(id: EntityId): Entity | null {
    return this.node.entities.get(id);
  }

  /**
   * Fetch multiple entities matching a query.
   */
  async fetchEntities(collection: CollectionId, args: unknown): Promise<Entity[]> {
    const matchArgs = args as MatchArgs;
    this.node.policyAgent.canAccessCollection(this.cdata, collection);
    return this.node.fetchEntitiesFromLocal(collection, matchArgs.selection);
  }

  /**
   * Commit a local transaction — the full commit pipeline.
   *
   * Rust: `commit_local_trx()` in context.rs (NodeAndContext impl)
   *
   * Phases:
   * 1. Prevent double-commit (atomic alive check)
   * 2. Generate commit events from each entity
   * 3. Policy validation and attestation
   * 4. Store events and update heads
   * 5. Persist canonical state
   *
   * Note: Peer replication (Phase 5 in Rust) and reactor notification (Phase 7)
   * are deferred until those subsystems are ported.
   */
  async commitLocalTrx(trx: Transaction): Promise<void> {
    // Phase 1: Prevent double-commit
    if (!trx.alive.value) {
      throw MutationError.general(new Error('Transaction already committed or rolled back'));
    }
    trx.alive.value = false;

    // Phase 2: Generate events from transaction entities
    const entityEvents: Array<{ entity: Entity; event: Event }> = [];
    for (const entity of trx.entities) {
      const event = entity.generateCommitEvent();
      if (event) {
        // Validate creation events
        if (event.isEntityCreate()) {
          if (!trx.createdEntityIds.has(entity.id().toString())) {
            throw MutationError.general(new Error(
              `Cannot commit phantom entity ${entity.id()}: entity has empty parent ` +
              `(creation event) but was not created in this transaction via create()`,
            ));
          }
        }
        entityEvents.push({ entity, event });
      }
    }

    // Phase 3: Policy validation and attestation
    const attestedEvents: Array<{
      entity: Entity;
      attested: Attested<Event>;
    }> = [];

    for (const { entity, event } of entityEvents) {
      // Get the canonical (upstream) entity for before-state
      const entityBefore = entity.kind.type === 'Transacted'
        ? entity.kind.upstream
        : entity;

      // Create a temporary fork to apply the event for after-state validation
      const forked = entity.snapshot({ value: true });
      forked.applyEvent(event);

      // Check policy
      const attestation = this.node.policyAgent.checkEvent(
        this.cdata,
        entityBefore,
        forked,
        event,
      );

      const { Attested } = require('@ankurah/proto');
      const attested = Attested.opt(event, attestation);
      attestedEvents.push({ entity, attested });
    }

    // Phase 4: Store events and update heads
    for (const { entity, attested } of attestedEvents) {
      const collection = await this.node.storageEngine.collection(attested.payload.collection);
      await collection.addEvent(attested);
      entity.commitHead(Clock.fromEventId(attested.payload.id()));
    }

    // Phase 5: Persist canonical state (apply events to upstream entities)
    for (const { entity, attested } of attestedEvents) {
      const collection = await this.node.storageEngine.collection(attested.payload.collection);

      // Apply event to canonical entity
      let canonicalEntity: Entity;
      if (entity.kind.type === 'Transacted') {
        const upstream = entity.kind.upstream;
        upstream.applyEvent(attested.payload);
        canonicalEntity = upstream;
      } else {
        canonicalEntity = entity;
      }

      const state = canonicalEntity.toState();
      const entityState = new EntityState(
        canonicalEntity.id(),
        canonicalEntity.collection(),
        state,
      );

      const attestation = this.node.policyAgent.attestState(entityState);
      const { Attested } = require('@ankurah/proto');
      const attestedState = Attested.opt(entityState, attestation);
      await collection.setState(attestedState);
    }

    // Phase 6: Peer replication — deferred until connector layer is ported
    // Phase 7: Reactor notification — deferred until reactor is ported
  }
}
