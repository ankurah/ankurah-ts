// MIRRORS: ankurah/core/src/node.rs
import {
  type CollectionId,
  type EntityId,
  EntityId as EntityIdClass,
  type Attested,
  type Event,
  Clock,
  EntityState,
} from '@ankurah/proto';
import { Selection, type Predicate, parseSelection } from '@ankurah/ankql';

import { Entity, WeakEntitySet } from './entity.ts';
import { Context, type TContext } from './context.ts';
import type { Transaction } from './transaction.ts';
import { MutationError, RetrievalError } from './error.ts';
import { EntityChange } from './changes.ts';
import type { StorageEngine, StorageCollection } from './storage.ts';
import type { PolicyAgent } from './policy.ts';
import { CollectionSet } from './collectionset.ts';
import { Reactor } from './reactor/index.ts';
import { EntityLiveQuery } from './livequery.ts';
import { TypeResolver } from './type_resolver.ts';

// ── PeerState ────────────────────────────────────────────────────────────────
// Rust: pub struct PeerState { sender, _durable, subscription_handler, pending_requests, pending_updates }
// Deferred: Layer 7 (peer networking). PeerState will be ported with connector layer.

// ── MatchArgs ────────────────────────────────────────────────────────────────
// Rust: pub struct MatchArgs { pub selection: Selection, pub cached: bool }

export interface MatchArgs {
  selection: Selection;
  cached: boolean;
}

// Rust: impl TryInto<MatchArgs> for &str / String
export function matchArgs(selection: Selection | string, cached = true): MatchArgs {
  if (typeof selection === 'string') {
    return { selection: parseSelection(selection), cached };
  }
  return { selection, cached };
}

// Rust: impl From<Predicate> for MatchArgs
export function matchArgsFromPredicate(predicate: Predicate): MatchArgs {
  return { selection: Selection.fromPredicate(predicate), cached: true };
}

// Rust: pub fn nocache<T>(s: T) -> Result<MatchArgs, ParseError>
export function nocache(selection: Selection | string): MatchArgs {
  if (typeof selection === 'string') {
    return { selection: parseSelection(selection), cached: false };
  }
  return { selection, cached: false };
}

// ── EntitySubscriptionState ──────────────────────────────────────────────────
// Rust: struct EntitySubscriptionState<CD: ContextData> { tracked: BTreeMap<CollectionId, BTreeMap<EntityId, CD>> }
// Deferred: Layer 7 (peer networking).

// ── Node ─────────────────────────────────────────────────────────────────────
// Rust: pub struct Node<SE, PA>(pub(crate) Arc<NodeInner<SE, PA>>)
// Divergence: Rust uses Arc<NodeInner> with Deref; TS uses a plain class [E8].
// Divergence: Rust is generic over StorageEngine and PolicyAgent; TS uses interface fields [A6].

export class Node {
  readonly id: EntityId;
  readonly durable: boolean;
  readonly collections: CollectionSet;
  readonly entities: WeakEntitySet;
  // Rust: peer_connections: SafeMap<EntityId, Arc<PeerState>>
  // Deferred: Layer 7 (peer networking)
  // Rust: durable_peers: SafeSet<EntityId>
  // Deferred: Layer 7 (peer networking)
  readonly reactor: Reactor;
  readonly policyAgent: PolicyAgent<unknown>;
  // Rust: pub system: SystemManager<SE, PA>
  // Deferred: SystemManager ported separately
  // Rust: pub(crate) subscription_relay: Option<SubscriptionRelay<...>>
  // Deferred: Layer 7 (peer networking)
  // Rust: pub(crate) type_resolver: TypeResolver
  readonly typeResolver: TypeResolver;

  /** Storage engine reference (for direct access where CollectionSet isn't used) */
  readonly storageEngine: StorageEngine;

  /** Context data factory — creates context data for new contexts */
  private readonly defaultContextData: unknown;

  // Rust: pub fn new(engine: Arc<SE>, policy_agent: PA) -> Self
  // Divergence: Takes options object instead of positional args [E8].
  constructor(options: {
    id?: EntityId;
    durable?: boolean;
    storageEngine: StorageEngine;
    policyAgent: PolicyAgent<unknown>;
    contextData?: unknown;
    reactor?: Reactor;
  }) {
    this.id = options.id ?? EntityIdClass.new();
    this.durable = options.durable ?? false;
    this.storageEngine = options.storageEngine;
    this.collections = new CollectionSet(options.storageEngine);
    this.entities = new WeakEntitySet();
    this.policyAgent = options.policyAgent;
    this.reactor = options.reactor ?? new Reactor();
    this.typeResolver = new TypeResolver();
    this.defaultContextData = options.contextData ?? null;
  }

  // Rust: pub fn weak(&self) -> WeakNode<SE, PA>
  // Deferred: WeakNode not needed in single-threaded JS [E8].

  // ── Peer networking (Layer 7 — deferred) ────────────────────────────
  // register_peer, deregister_peer, request, send_update
  // handle_message, handle_request, handle_update
  // relay_to_required_peers, commit_remote_transaction
  // generate_entity_delta, collect_event_bridge
  // get_from_peer, get_durable_peer_random, get_durable_peers
  // ensure_entity_subscription, subscribe_entities_with_peer
  // resubscribe_tracked_entities_to_peer, flush_dead_entity_subscriptions

  // Rust: pub fn next_entity_id(&self) -> EntityId
  nextEntityId(): EntityId {
    return EntityIdClass.new();
  }

  // Rust: pub fn context(&self, data: PA::ContextData) -> Result<Context, Error>
  // Divergence: No system readiness check yet (SystemManager deferred) [E8].
  context(contextData?: unknown): Context {
    const cdata = contextData ?? this.defaultContextData;
    const nodeContext = new NodeAndContext(this, cdata);
    return new Context(nodeContext);
  }

  // Rust: pub async fn context_async(&self, data: PA::ContextData) -> Context
  // Deferred: Requires SystemManager.waitSystemReady()

  // Rust: pub(crate) async fn fetch_entities_from_local(...)
  async fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Entity[]> {
    const collection = await this.collections.get(collectionId);
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

  // Rust: impl fmt::Display for Node
  toString(): string {
    return `Node(${this.id.toBase64Short()})`;
  }
}

// ── TNodeErased ──────────────────────────────────────────────────────────────
// Rust: pub trait TNodeErased<E: AbstractEntity + Filterable + Send + 'static>
// Already represented as ReactorNodeLike (reactor/index.ts) and NodeLike (reactor/fetch_gap.ts).
// Node structurally conforms to ReactorNodeLike.

// ── NodeAndContext ───────────────────────────────────────────────────────────
// Rust: pub struct NodeAndContext<SE, PA: PolicyAgent> { pub node: Node<SE, PA>, pub cdata: PA::ContextData }
// Rust: impl TContext for NodeAndContext<SE, PA>
// Divergence: TS Node is not generic; NodeAndContext uses interface fields [E7].

export class NodeAndContext implements TContext {
  readonly node: Node;
  readonly cdata: unknown;

  constructor(node: Node, cdata: unknown) {
    this.node = node;
    this.cdata = cdata;
  }

  // ── TContext interface ──────────────────────────────────────────────

  // Rust: fn node_id(&self) -> EntityId { self.node.id }
  nodeId(): EntityId {
    return this.node.id;
  }

  // Rust: fn create_entity(&self, collection, trx_alive) -> Entity
  createEntity(collection: CollectionId, trxAlive: { value: boolean }): Entity {
    const primaryEntity = this.node.entities.create(collection);
    return primaryEntity.snapshot(trxAlive);
  }

  // Rust: fn check_write(&self, entity) -> Result<(), AccessDenied>
  checkWrite(entity: Entity): void {
    this.node.policyAgent.checkWrite(this.cdata, entity, null);
  }

  // Rust: async fn get_entity(&self, id, collection, cached) -> Result<Entity, RetrievalError>
  // Simplified vs Rust: no peer fetching, just local storage lookup.
  // Full peer-assisted retrieval will be added when connectors are ported.
  async getEntity(id: EntityId, collection: CollectionId, cached: boolean): Promise<Entity> {
    // Check local resident entities first
    const local = this.node.entities.get(id);
    if (local) {
      return local;
    }

    // Fetch from storage
    // Rust: full get_entity in context.rs with peer fallback and should_fallback_to_local
    // Deferred: peer networking (Layer 7)
    const storageCollection = await this.node.collections.get(collection);
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

  // Rust: fn get_resident_entity(&self, id) -> Option<Entity>
  getResidentEntity(id: EntityId): Entity | null {
    return this.node.entities.get(id);
  }

  // Rust: async fn fetch_entities(&self, collection, args) -> Result<Vec<Entity>, RetrievalError>
  async fetchEntities(collection: CollectionId, args: MatchArgs): Promise<Entity[]> {
    this.node.policyAgent.canAccessCollection(this.cdata as unknown[], collection);

    // Rust: args.selection = self.node.type_resolver.resolve_selection_types(args.selection);
    args.selection = this.node.typeResolver.resolveSelectionTypes(args.selection);

    // Rust: if !self.node.durable { ... fetch_from_peer ... } else { ... from local ... }
    // Simplified: always fetch from local (peer networking deferred)
    return this.node.fetchEntitiesFromLocal(collection, args.selection);
  }

  // Rust: fn query(&self, collection_id, args) -> Result<EntityLiveQuery, RetrievalError>
  query(collectionId: CollectionId, args: MatchArgs): EntityLiveQuery {
    return EntityLiveQuery.create(this.node, collectionId, args, this.cdata);
  }

  // Rust: async fn collection(&self, id) -> Result<StorageCollectionWrapper, RetrievalError>
  // Divergence: Returns StorageCollection directly instead of StorageCollectionWrapper [E7].
  async collection(id: CollectionId): Promise<StorageCollection> {
    return this.node.collections.get(id);
  }

  // Rust: async fn commit_local_trx(&self, trx) -> Result<(), MutationError>
  // Full 7-phase commit pipeline (from context.rs impl TContext for NodeAndContext)
  async commitLocalTrx(trx: Transaction): Promise<void> {
    // Phase 1: Prevent double-commit
    // Rust: compare_exchange on AtomicBool
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

    // Phase 4: Store events and update heads BEFORE relaying (makes entities visible to server echo)
    for (const { entity, attested } of attestedEvents) {
      const collection = await this.node.collections.get(attested.payload.collection);
      await collection.addEvent(attested);
      entity.commitHead(Clock.fromEventId(attested.payload.id()));
    }

    // Phase 5: Relay to peers and wait for confirmation
    // Rust: self.node.relay_to_required_peers(&self.cdata, trx_id, &attested_events).await?;
    // Deferred: peer networking (Layer 7)

    // Phase 6: Persist canonical state + collect EntityChanges
    const entityChanges: EntityChange[] = [];
    for (const { entity, attested } of attestedEvents) {
      const collectionId = attested.payload.collection;
      const collection = await this.node.collections.get(collectionId);

      // Apply event to canonical entity (upstream for transactional forks, entity itself for primary)
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

      // Collect entity change for reactor notification
      entityChanges.push(EntityChange.create(canonicalEntity, [attested]));
    }

    // Phase 7: Notify reactor of ALL changes
    await this.node.reactor.notifyChange(entityChanges);
  }
}

// ── NodeAndContext private methods ───────────────────────────────────────────
// Rust: impl NodeAndContext { should_fallback_to_local, get_entity, fetch_entities, commit_local_trx, fetch_from_peer }
// should_fallback_to_local and fetch_from_peer deferred to Layer 7 (peer networking).
// get_entity, fetch_entities, commit_local_trx implemented above as TContext methods.
