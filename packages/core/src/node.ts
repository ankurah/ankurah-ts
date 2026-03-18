// MIRRORS: ankurah/core/src/node.rs
import {
  type CollectionId,
  type EntityId,
  EntityId as EntityIdClass,
  type Attested,
  type Event,
  Clock,
  EntityState,
  Attested as AttestedClass,
  NodeMessage,
  NodeRequest,
  NodeResponse,
  NodeResponseBody,
  NodeRequestBody,
  NodeUpdate,
  NodeUpdateAck,
  NodeUpdateAckBody,
  NodeUpdateBody,
  RequestId,
  TransactionId,
  UpdateId,
  type QueryId,
  EntityDelta,
  DeltaContent,
  StateFragment,
  EventFragment,
  KnownEntity,
  Presence,
} from '@ankurah/proto';
import { Selection, type Predicate, parseSelection } from '@ankurah/ankql';

import { Entity, WeakEntitySet } from './entity.ts';
import { Context, type TContext } from './context.ts';
import type { Transaction } from './transaction.ts';
import { MutationError, RetrievalError, RequestError } from './error.ts';
import { EntityChange } from './changes.ts';
import type { StorageEngine, StorageCollection } from './storage.ts';
import type { PolicyAgent } from './policy.ts';
import { CollectionSet } from './collectionset.ts';
import { Reactor } from './reactor/index.ts';
import { EntityLiveQuery } from './livequery.ts';
import { TypeResolver } from './type_resolver.ts';
import type { PeerSender, NodeComms } from './connector.ts';
import { SendError } from './connector.ts';
import { SystemManager } from './system.ts';
import { SubscriptionHandler } from './peer_subscription/server.ts';
import { NodeApplier } from './node_applier.ts';
import { LocalRetriever } from './retrieval.ts';
import { expandStates } from './util/expand_states.ts';
import { spawn } from './task.ts';

// ── PeerState ────────────────────────────────────────────────────────────────
// Rust: pub struct PeerState { sender, _durable, subscription_handler, pending_requests, pending_updates }

interface PeerState {
  sender: PeerSender;
  durable: boolean;
  subscriptionHandler: SubscriptionHandler;
  pendingRequests: Map<string, {
    resolve: (body: NodeResponseBody) => void;
    reject: (err: Error) => void;
  }>;
  pendingUpdates: Map<string, {
    resolve: (body: NodeResponseBody) => void;
    reject: (err: Error) => void;
  }>;
}

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

export class Node implements NodeComms {
  readonly id: EntityId;
  readonly durable: boolean;
  readonly collections: CollectionSet;
  readonly entities: WeakEntitySet;
  // Rust: peer_connections: SafeMap<EntityId, Arc<PeerState>>
  private readonly peerConnections: Map<string, PeerState> = new Map();
  // Rust: durable_peers: SafeSet<EntityId>
  private readonly durablePeers: Set<string> = new Set();
  readonly reactor: Reactor;
  readonly policyAgent: PolicyAgent<unknown>;
  // Rust: pub system: SystemManager<SE, PA>
  readonly system: SystemManager;
  // Rust: pub(crate) subscription_relay: Option<SubscriptionRelay<...>>
  // Deferred: SubscriptionRelay integration — SubscriptionHandler handles server-side
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
    // Rust: SystemManager::new(collections, entityset, reactor, durable)
    this.system = new SystemManager(this.collections, this.entities, this.reactor, this.durable);
  }

  // ── NodeComms interface ──────────────────────────────────────────────
  // Rust: impl NodeComms for Node<SE, PA>

  // NodeComms.id()
  nodeId(): EntityId {
    return this.id;
  }

  // NodeComms.durable()
  isDurable(): boolean {
    return this.durable;
  }

  // NodeComms.systemRoot()
  systemRoot(): Attested<EntityState> | null {
    return this.system.root();
  }

  // NodeComms.cloned()
  cloned(): NodeComms {
    return this;
  }

  // Rust: pub fn register_peer(&self, presence, sender)
  registerPeer(presence: Presence, sender: PeerSender): void {
    const subscriptionHandler = new SubscriptionHandler(presence.nodeId, this);
    this.peerConnections.set(presence.nodeId.toBase64(), {
      sender,
      durable: presence.durable,
      subscriptionHandler,
      pendingRequests: new Map(),
      pendingUpdates: new Map(),
    });

    if (presence.durable) {
      this.durablePeers.add(presence.nodeId.toBase64());

      if (!this.durable) {
        if (presence.systemRoot !== null) {
          const systemRoot = presence.systemRoot;
          spawn((async () => {
            try {
              await this.system.joinSystem(systemRoot);
            } catch (e) {
              console.error(`Node(${this.id.toBase64Short()}) failed to join system: ${e}`);
            }
          })());
        } else {
          console.error(`Node(${this.id.toBase64Short()}) durable peer ${presence.nodeId.toBase64Short()} has no system root`);
        }
      }
    }
  }

  // Rust: pub fn deregister_peer(&self, node_id)
  deregisterPeer(nodeId: EntityId): void {
    this.durablePeers.delete(nodeId.toBase64());
    // Get and cleanup subscriptions before removing the peer
    const peerState = this.peerConnections.get(nodeId.toBase64());
    if (peerState) {
      this.peerConnections.delete(nodeId.toBase64());
      // ReactorSubscription is automatically unsubscribed when SubscriptionHandler is garbage collected
    }
  }

  // Rust: pub async fn handle_message(&self, message: NodeMessage) -> Result<()>
  async handleMessage(message: NodeMessage): Promise<void> {
    message.match({
      Update: (v) => {
        const update = v.update;
        const senderPeer = this.peerConnections.get(update.from.toBase64());
        if (senderPeer) {
          if (!update.to.equals(this.id)) {
            console.warn(`${this.id} received message from ${update.from} but is not the intended recipient`);
            return;
          }

          const id = update.id;
          const to = update.from;
          const from = this.id;

          // Handle the update
          this.handleUpdate(update).then(() => {
            senderPeer.sender.sendMessage(
              new NodeMessage('UpdateAck', {
                updateAck: new NodeUpdateAck(id, from, to, new NodeUpdateAckBody('Success', {})),
              }),
            );
          }).catch((e) => {
            senderPeer.sender.sendMessage(
              new NodeMessage('UpdateAck', {
                updateAck: new NodeUpdateAck(id, from, to, new NodeUpdateAckBody('Error', { message: String(e) })),
              }),
            );
          });
        }
      },
      UpdateAck: (_v) => {
        // Acknowledgement received - currently a no-op
      },
      Request: (v) => {
        const { auth, request } = v;
        const senderPeer = this.peerConnections.get(request.from.toBase64());
        if (senderPeer) {
          const from = request.from;
          const requestId = request.id;
          if (!request.to.equals(this.id)) {
            console.warn(`${this.id} received message from ${request.from} but is not the intended recipient`);
            return;
          }

          // Validate the request auth and handle
          this.policyAgent.checkRequest(auth, request).then(async (cdata) => {
            try {
              const body = await this.handleRequest(cdata, request);
              senderPeer.sender.sendMessage(
                new NodeMessage('Response', {
                  response: new NodeResponse(requestId, this.id, from, body),
                }),
              );
            } catch (e) {
              senderPeer.sender.sendMessage(
                new NodeMessage('Response', {
                  response: new NodeResponse(
                    requestId, this.id, from,
                    new NodeResponseBody('Error', { message: String(e) }),
                  ),
                }),
              );
            }
          }).catch((e) => {
            senderPeer.sender.sendMessage(
              new NodeMessage('Response', {
                response: new NodeResponse(
                  requestId, this.id, from,
                  new NodeResponseBody('Error', { message: String(e) }),
                ),
              }),
            );
          });
        }
      },
      Response: (v) => {
        const response = v.response;
        const peerState = this.peerConnections.get(response.from.toBase64());
        if (peerState) {
          const pending = peerState.pendingRequests.get(response.requestId.toUlidString());
          if (pending) {
            peerState.pendingRequests.delete(response.requestId.toUlidString());
            pending.resolve(response.body);
          }
        }
      },
      UnsubscribeQuery: (v) => {
        const peerState = this.peerConnections.get(v.from.toBase64());
        if (peerState) {
          peerState.subscriptionHandler.removePredicate(v.queryId);
        }
      },
      UnsubscribeEntities: (_v) => {
        // Deferred: entity-level unsubscribe
      },
    });
  }

  // Rust: pub async fn request(&self, node_id, cdata, request_body) -> Result<NodeResponseBody, RequestError>
  async request(
    nodeId: EntityId,
    cdata: unknown,
    requestBody: NodeRequestBody,
  ): Promise<NodeResponseBody> {
    const connection = this.peerConnections.get(nodeId.toBase64());
    if (!connection) {
      throw new RequestError('PeerNotConnected', `Peer ${nodeId.toBase64Short()} not connected`);
    }

    const requestId = RequestId.new();
    const request = new NodeRequest(requestId, nodeId, this.id, requestBody);
    const auth = this.policyAgent.signRequest(cdata as unknown[], request);

    return new Promise<NodeResponseBody>((resolve, reject) => {
      connection.pendingRequests.set(requestId.toUlidString(), { resolve, reject });
      try {
        connection.sender.sendMessage(
          new NodeMessage('Request', { auth, request }),
        );
      } catch (e) {
        connection.pendingRequests.delete(requestId.toUlidString());
        reject(new RequestError('SendError', `Failed to send request: ${e}`));
      }
    });
  }

  // Rust: pub fn send_update(&self, node_id, notification)
  sendUpdate(nodeId: EntityId, notification: NodeUpdateBody): void {
    const connection = this.peerConnections.get(nodeId.toBase64());
    if (!connection) {
      console.warn(`Failed to send update to peer ${nodeId}: PeerNotConnected`);
      return;
    }

    const id = UpdateId.new();
    const message = new NodeMessage('Update', {
      update: new NodeUpdate(id, this.id, nodeId, notification),
    });

    try {
      connection.sender.sendMessage(message);
    } catch (e) {
      console.warn(`Failed to send update to peer ${nodeId}: ${e}`);
    }
  }

  // Rust: async fn handle_request(&self, cdata, request) -> Result<NodeResponseBody>
  private async handleRequest(cdata: unknown, request: NodeRequest): Promise<NodeResponseBody> {
    return request.body.match({
      CommitTransaction: async (v) => {
        try {
          await this.commitRemoteTransaction(cdata, v.id, v.events);
          return new NodeResponseBody('CommitComplete', { id: v.id });
        } catch (e) {
          return new NodeResponseBody('Error', { message: String(e) });
        }
      },
      Fetch: async (v) => {
        this.policyAgent.canAccessCollection(cdata as unknown[], v.collection);
        const storageCollection = await this.collections.get(v.collection);
        // Divergence: Selection stored as opaque bytes in wire format;
        // for local-process connector we pass the Selection object through directly.
        // The LocalProcessSender callbacks deliver messages in-process, so no serialization happens.
        // This means v.selection is actually a Selection object (not Uint8Array) at runtime [E18].
        const selection = v.selection as unknown as Selection;
        const filteredSelection = new Selection(
          this.policyAgent.filterPredicate(cdata as unknown[], v.collection, selection.predicate),
          selection.orderBy,
          selection.limit,
        );

        const expandedStates = await expandStates(
          await storageCollection.fetchStates(filteredSelection),
          v.knownMatches.map((k: KnownEntity) => k.entityId),
          storageCollection,
        );

        const knownMap = new Map<string, import('@ankurah/proto').Clock>();
        for (const k of v.knownMatches) {
          knownMap.set(k.entityId.toBase64(), k.head);
        }

        const deltas: EntityDelta[] = [];
        for (const state of expandedStates) {
          const delta = await this.generateEntityDelta(knownMap, state, storageCollection);
          if (delta !== null) {
            deltas.push(delta);
          }
        }
        return new NodeResponseBody('Fetch', { deltas });
      },
      Get: async (v) => {
        this.policyAgent.canAccessCollection(cdata as unknown[], v.collection);
        const storageCollection = await this.collections.get(v.collection);
        const states: Attested<EntityState>[] = [];
        for (const id of v.ids) {
          try {
            const state = await storageCollection.getState(id);
            states.push(state);
          } catch (_e) {
            // Entity not found — skip
          }
        }
        return new NodeResponseBody('Get', { states });
      },
      GetEvents: async (v) => {
        this.policyAgent.canAccessCollection(cdata as unknown[], v.collection);
        const storageCollection = await this.collections.get(v.collection);
        const events = await storageCollection.getEvents(v.eventIds);
        return new NodeResponseBody('GetEvents', { events });
      },
      SubscribeQuery: async (v) => {
        const peerState = this.peerConnections.get(request.from.toBase64());
        if (!peerState) {
          throw new Error(`Peer ${request.from} not connected`);
        }
        // Divergence: Selection stored as opaque bytes in wire format; at runtime it's a Selection [E18]
        const selection = v.selection as unknown as Selection;
        return peerState.subscriptionHandler.subscribeQuery(
          this, v.queryId, v.collection, selection, v.version, v.knownMatches,
        );
      },
      SubscribeEntity: async (_v) => {
        throw new Error('SubscribeEntity not yet implemented');
      },
    });
  }

  // Rust: async fn handle_update(&self, notification) -> Result<()>
  private async handleUpdate(notification: NodeUpdate): Promise<void> {
    const connection = this.peerConnections.get(notification.from.toBase64());
    if (!connection) {
      throw new Error(`Rejected notification from unknown node ${notification.from}`);
    }

    notification.body.match({
      SubscriptionUpdate: async (v) => {
        await NodeApplier.applyUpdates(this, notification.from, v.items);
      },
    });
  }

  // Rust: pub async fn relay_to_required_peers(&self, cdata, id, events) -> Result<(), MutationError>
  async relayToRequiredPeers(
    cdata: unknown,
    id: TransactionId,
    events: Attested<Event>[],
  ): Promise<void> {
    for (const peerIdStr of this.durablePeers) {
      const peerState = this.peerConnections.get(peerIdStr);
      if (!peerState) continue;
      const peerId = peerState.sender.recipientNodeId();

      const body = await this.request(
        peerId,
        cdata,
        new NodeRequestBody('CommitTransaction', { id, events }),
      );

      if (body.is('CommitComplete')) {
        // Success
      } else if (body.is('Error')) {
        throw MutationError.general(new Error(`Peer ${peerId} rejected: ${body.value.message}`));
      } else {
        throw MutationError.general(new Error(`Peer ${peerId} returned unexpected response`));
      }
    }
  }

  // Rust: pub async fn commit_remote_transaction(&self, cdata, id, events) -> Result<(), MutationError>
  async commitRemoteTransaction(
    cdata: unknown,
    _id: TransactionId,
    events: Attested<Event>[],
  ): Promise<void> {
    const changes: EntityChange[] = [];

    for (const attestedEvent of events) {
      const event = attestedEvent.payload;
      const collection = await this.collections.get(event.collection);
      const retriever = new LocalRetriever(collection);

      // Get or create entity
      const local = this.entities.get(event.entityId);
      let entity: Entity;
      if (local) {
        entity = local;
      } else {
        try {
          const state = await retriever.getState(event.entityId);
          if (state !== null) {
            const [_changed, e] = this.entities.withState(event.entityId, event.collection, state.payload.state);
            entity = e;
          } else {
            entity = Entity.create(event.entityId, event.collection);
            this.entities.register(entity);
          }
        } catch {
          entity = Entity.create(event.entityId, event.collection);
          this.entities.register(entity);
        }
      }

      // Validate event vs entity state
      // Rust: if event.is_entity_create() && entity.head().is_empty() { create path }
      //       else { update path with lineage validation }
      if (event.isEntityCreate()) {
        if (!entity.head().isEmpty()) {
          // Entity already exists — reject duplicate create
          throw MutationError.general(new Error(
            `Cannot create entity ${event.entityId}: entity already exists`,
          ));
        }
      } else {
        if (entity.head().isEmpty()) {
          // Update for nonexistent entity — parent references events that don't exist
          throw MutationError.general(new Error(
            `Cannot update entity ${event.entityId}: entity does not exist (nonexistent parent lineage)`,
          ));
        }
      }

      // Apply the event
      entity.applyEvent(event);

      // Store event
      await collection.addEvent(attestedEvent);

      // Store state
      const state = entity.toState();
      const entityState = new EntityState(entity.id(), entity.collection(), state);
      const attestation = this.policyAgent.attestState(entityState);
      const attested = AttestedClass.opt(entityState, attestation);
      await collection.setState(attested);

      changes.push(EntityChange.create(entity, [attestedEvent]));
    }

    await this.reactor.notifyChange(changes);
  }

  // Rust: pub(crate) async fn generate_entity_delta(known_map, entity_state, storage_collection) -> Result<Option<EntityDelta>>
  async generateEntityDelta(
    knownMap: Map<string, Clock>,
    entityState: Attested<EntityState>,
    storageCollection: StorageCollection,
  ): Promise<EntityDelta | null> {
    const entityId = entityState.payload.entityId;
    const collection = entityState.payload.collection;
    const currentHead = entityState.payload.state.head;

    const knownHead = knownMap.get(entityId.toBase64());
    if (knownHead) {
      // Heads equal — omit (client already has current state)
      if (knownHead.equals(currentHead)) {
        return null;
      }

      // Rust: uses collect_event_bridge() with full lineage comparison (Comparison + EventAccumulator)
      // to walk the event DAG and collect all events between known_head and current_head.
      // Since the lineage module is not yet ported, we fall through to StateSnapshot which is
      // always correct (just not as bandwidth-efficient as EventBridge).
      // TODO: Implement collect_event_bridge() once the lineage module is ported.
    }

    // Default: StateSnapshot
    const stateFragment = StateFragment.fromAttestedEntityState(entityState);
    return new EntityDelta(entityId, collection, new DeltaContent('StateSnapshot', { state: stateFragment }));
  }

  // Rust: fn get_durable_peers(&self) -> Vec<EntityId>
  getDurablePeers(): EntityId[] {
    const peers: EntityId[] = [];
    for (const peerIdStr of this.durablePeers) {
      const peerState = this.peerConnections.get(peerIdStr);
      if (peerState) {
        peers.push(peerState.sender.recipientNodeId());
      }
    }
    return peers;
  }

  // Rust: pub fn next_entity_id(&self) -> EntityId
  nextEntityId(): EntityId {
    return EntityIdClass.new();
  }

  // Rust: pub fn context(&self, data: PA::ContextData) -> Result<Context, Error>
  // Divergence: Rust checks system readiness and returns Err if not ready;
  // TS does not enforce this for backwards compatibility with existing tests [E8].
  context(contextData?: unknown): Context {
    const cdata = contextData ?? this.defaultContextData;
    const nodeContext = new NodeAndContext(this, cdata);
    return new Context(nodeContext);
  }

  // Rust: pub async fn context_async(&self, data: PA::ContextData) -> Context
  async contextAsync(contextData?: unknown): Promise<Context> {
    await this.system.waitSystemReady();
    const cdata = contextData ?? this.defaultContextData;
    const nodeContext = new NodeAndContext(this, cdata);
    return new Context(nodeContext);
  }

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
  async getEntity(id: EntityId, collection: CollectionId, cached: boolean): Promise<Entity> {
    // Check local resident entities first
    const local = this.node.entities.get(id);
    if (local) {
      return local;
    }

    // Try local storage first
    const storageCollection = await this.node.collections.get(collection);
    try {
      const entityState = await storageCollection.getState(id);
      const [_changed, entity] = this.node.entities.withState(
        id,
        collection,
        entityState.payload.state,
      );
      return entity;
    } catch (_localError) {
      // Not found locally — try peer
    }

    // Rust: get_from_peer for non-durable nodes
    if (!this.node.durable) {
      const durablePeers = this.node.getDurablePeers();
      for (const peerId of durablePeers) {
        try {
          const response = await this.node.request(
            peerId,
            this.cdata,
            new NodeRequestBody('Get', { collection, ids: [id] }),
          );
          if (response.is('Get') && response.value.states.length > 0) {
            const state = response.value.states[0];
            const [_changed, entity] = this.node.entities.withState(
              id,
              collection,
              state.payload.state,
            );
            // Also persist to local storage
            await storageCollection.setState(state);
            return entity;
          }
        } catch (_peerError) {
          // Try next peer
        }
      }
    }

    throw RetrievalError.entityNotFound(id);
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

    // Rust: if !self.node.durable { fetch_from_peer } else { from local }
    if (!this.node.durable) {
      // Try fetching from a durable peer
      const durablePeers = this.node.getDurablePeers();
      if (durablePeers.length > 0) {
        const peerId = durablePeers[0];
        try {
          // Rust: Pre-fetch known_matches from local storage
          const knownMatchedEntities = await this.node.fetchEntitiesFromLocal(collection, args.selection);
          const knownMatches = knownMatchedEntities.map(
            (entity) => new KnownEntity(entity.id(), entity.head()),
          );

          // Divergence: Selection stored as opaque bytes in wire format;
          // for local-process connector, we pass the Selection object directly [E18]
          const response = await this.node.request(
            peerId,
            this.cdata,
            new NodeRequestBody('Fetch', {
              collection,
              selection: args.selection as unknown as Uint8Array,
              knownMatches,
            }),
          );
          if (response.is('Fetch')) {
            // Apply deltas to local storage
            const retriever = new LocalRetriever(await this.node.collections.get(collection));
            await NodeApplier.applyDeltas(this.node, peerId, response.value.deltas, retriever);
            await retriever.storeUsedEvents();

            // Now fetch from local
            return this.node.fetchEntitiesFromLocal(collection, args.selection);
          }
        } catch (e) {
          // Fallback to local if peer fetch fails
          if (args.cached) {
            return this.node.fetchEntitiesFromLocal(collection, args.selection);
          }
          throw e instanceof RetrievalError ? e : new RetrievalError('RequestError', String(e));
        }
      }
    }

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

      const attested = AttestedClass.opt(event, attestation);
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
    if (this.node.getDurablePeers().length > 0) {
      const trxId = TransactionId.new();
      await this.node.relayToRequiredPeers(
        this.cdata,
        trxId,
        attestedEvents.map(ae => ae.attested),
      );
    }

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
      const attestedState = AttestedClass.opt(entityState, attestation);
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
