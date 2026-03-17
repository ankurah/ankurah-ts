// MIRRORS: ankurah/core/src/entity.rs

import {
  type CollectionId,
  type EntityId,
  EntityId as EntityIdClass,
  type EventId,
  Clock,
  State,
  StateBuffers,
  EntityState,
  Event,
  OperationSet,
  type Operation,
} from '@ankurah/proto';
import {
  Broadcast,
  type BroadcastId,
  type BroadcastListener,
  ListenerGuard as SignalListenerGuard,
  type Signal,
  type Listener,
  CurrentObserver,
} from '@ankurah/signals';

import type { PropertyBackend } from './property/backend/index.ts';
import { backendFromString, LWWBackend, YjsBackend } from './property/backend/index.ts';
import type { PropertyName } from './property/index.ts';
import type { Value } from './value/index.ts';
import { MutationError, RetrievalError, StateError } from './error.ts';

// ---------------------------------------------------------------------------
// EntityKind — Primary vs Transacted
// ---------------------------------------------------------------------------

/**
 * Tracks whether an entity is a canonical primary entity or a transaction fork.
 *
 * Rust: `pub enum EntityKind { Primary, Transacted { trx_alive, upstream } }`
 * Divergence: Rust uses Arc<AtomicBool> for trx_alive; TS uses shared { value: boolean } [E8].
 * Divergence: Rust uses Arc<EntityInner>; TS uses plain reference [E8].
 */
export type EntityKind =
  | { type: 'Primary' }
  | { type: 'Transacted'; trxAlive: { value: boolean }; upstream: Entity };

// ---------------------------------------------------------------------------
// EntityInnerState — mutable state behind RwLock in Rust
// ---------------------------------------------------------------------------

/**
 * Mutable state of an entity.
 *
 * Rust: `struct EntityInnerState { head: Clock, backends: BTreeMap<String, Arc<dyn PropertyBackend>> }`
 * Divergence: No RwLock needed — single-threaded JS [E8].
 * Divergence: No Arc<dyn PropertyBackend> — plain PropertyBackend references [E8].
 */
interface EntityInnerState {
  head: Clock;
  backends: Map<string, PropertyBackend>;
}

// ---------------------------------------------------------------------------
// Entity
// ---------------------------------------------------------------------------

/**
 * Core entity type. Holds identity, collection, property backends, and clock.
 *
 * Rust: `pub struct Entity(Arc<EntityInner>)`
 * Divergence: No Arc — plain class instance (JS single-threaded, GC handles memory) [E8].
 * Divergence: No Deref<Target = EntityInner> — methods defined directly on Entity [E7].
 */
export class Entity {
  /** Entity identity (immutable). Rust: `pub id: EntityId` */
  readonly entityId: EntityId;

  /** Collection this entity belongs to (immutable). Rust: `pub collection: CollectionId` */
  readonly collectionId: CollectionId;

  /** Entity kind: Primary or Transacted. Rust: `pub kind: EntityKind` */
  readonly kind: EntityKind;

  /** Mutable state (head clock + backends). Rust: `state: RwLock<EntityInnerState>` */
  private state: EntityInnerState;

  /**
   * Broadcast for change notifications.
   * Rust: `pub(crate) broadcast: ankurah_signals::broadcast::Broadcast`
   */
  readonly broadcast: Broadcast;

  private constructor(
    entityId: EntityId,
    collectionId: CollectionId,
    kind: EntityKind,
    state: EntityInnerState,
  ) {
    this.entityId = entityId;
    this.collectionId = collectionId;
    this.kind = kind;
    this.state = state;
    this.broadcast = new Broadcast();
  }

  // ── Construction ──────────────────────────────────────────────────

  /**
   * Create a brand new primary entity with empty state.
   *
   * Rust: `pub fn create(id: EntityId, collection: CollectionId) -> Self`
   */
  static create(id: EntityId, collection: CollectionId): Entity {
    return new Entity(
      id,
      collection,
      { type: 'Primary' },
      { head: Clock.default(), backends: new Map() },
    );
  }

  /**
   * Create an entity from persisted state.
   *
   * Rust: `fn from_state(id: EntityId, collection: CollectionId, state: &State) -> Result<Self, RetrievalError>`
   * Throws RetrievalError on failure.
   */
  static fromState(
    id: EntityId,
    collection: CollectionId,
    state: State,
  ): Entity {
    const backends = new Map<string, PropertyBackend>();
    for (const [name, buffer] of state.stateBuffers.entries()) {
      try {
        backends.set(name, backendFromString(name, buffer));
      } catch (e) {
        throw RetrievalError.deserializationError(
          e instanceof Error ? e : new Error(String(e)),
        );
      }
    }
    return new Entity(
      id,
      collection,
      { type: 'Primary' },
      { head: state.head, backends },
    );
  }

  // ── Identity & State Access ───────────────────────────────────────

  /**
   * Get the entity ID.
   *
   * Rust: EntityInner has `pub id: EntityId`
   */
  id(): EntityId {
    return this.entityId;
  }

  /**
   * Get the collection.
   *
   * Rust: EntityInner has `pub collection: CollectionId`
   */
  collection(): CollectionId {
    return this.collectionId;
  }

  /**
   * Get the current head clock.
   *
   * Rust: `pub fn head(&self) -> Clock`
   */
  head(): Clock {
    return this.state.head;
  }

  /**
   * Whether this entity can be mutated (i.e., belongs to a live transaction).
   *
   * Rust: `pub fn is_writable(&self) -> bool`
   */
  isWritable(): boolean {
    switch (this.kind.type) {
      case 'Primary':
        return false;
      case 'Transacted':
        return this.kind.trxAlive.value;
    }
  }

  // ── State Serialization ───────────────────────────────────────────

  /**
   * Serialize current entity state.
   *
   * Rust: `pub fn to_state(&self) -> Result<State, StateError>`
   * Throws StateError on failure.
   */
  toState(): State {
    const bufferMap = new Map<string, Uint8Array>();
    for (const [name, backend] of this.state.backends) {
      try {
        bufferMap.set(name, backend.toStateBuffer());
      } catch (e) {
        throw StateError.serializationError(
          e instanceof Error ? e : new Error(String(e)),
        );
      }
    }
    return new State(new StateBuffers(bufferMap), this.state.head);
  }

  /**
   * Serialize current entity state with identity.
   *
   * Rust: `pub fn to_entity_state(&self) -> Result<EntityState, StateError>`
   */
  toEntityState(): EntityState {
    const state = this.toState();
    return new EntityState(this.entityId, this.collectionId, state);
  }

  // ── Backend Access ────────────────────────────────────────────────

  /**
   * Get or lazily create a backend by its static class.
   *
   * Rust: `pub fn get_backend<P: PropertyBackend>(&self) -> Result<Arc<P>, RetrievalError>`
   * Divergence: TS uses class reference and string name lookup rather than trait dispatch [E8].
   */
  getBackend<P extends PropertyBackend>(
    backendClass: { propertyBackendName(): string; new (): P; fromStateBuffer(buffer: Uint8Array): P },
  ): P {
    const name = backendClass.propertyBackendName();
    let backend = this.state.backends.get(name);
    if (!backend) {
      backend = new backendClass();
      this.state.backends.set(name, backend);
    }
    return backend as P;
  }

  /**
   * Get a backend by name string, creating if needed.
   *
   * Rust: Uses backend_from_string() factory.
   */
  getBackendByName(name: string): PropertyBackend {
    let backend = this.state.backends.get(name);
    if (!backend) {
      backend = backendFromString(name);
      this.state.backends.set(name, backend);
    }
    return backend;
  }

  /**
   * Get a property value by field name. Searches all backends.
   * Used by View getters generated by defineModel().
   *
   * Rust: AbstractEntity::value() + Filterable::value() impl on Entity
   */
  getPropertyValue(fieldName: string): Value | null {
    if (fieldName === 'id') {
      return { type: 'EntityId', value: this.entityId };
    }
    for (const backend of this.state.backends.values()) {
      const value = backend.propertyValue(fieldName);
      if (value !== null) return value;
    }
    return null;
  }

  /**
   * Get an active type handle for a field. Used by Mutable getters generated by defineModel().
   * Returns the appropriate active type (LWW<T> or YrsString) for the field.
   *
   * This is used internally by defineModel() generated Mutable classes.
   */
  getActiveHandle(fieldName: string, backendKind: string): unknown {
    // Defer actual handle creation to the caller — this is a hook point.
    // The defineModel() Mutable getter calls this, then wraps appropriately.
    const backend = this.getBackendByName(backendKind);
    return { backend, fieldName, entity: this };
  }

  /**
   * Initialize a property value on the entity (for new entity creation).
   * Called by Model::initialize_new_entity().
   *
   * Rust: Delegates to InitializeWith trait impls per field type.
   */
  initializeProperty(
    fieldName: string,
    value: unknown,
    backendKind: string,
  ): void {
    const backend = this.getBackendByName(backendKind);

    if (backendKind === 'lww') {
      // Convert value to Value type and set on LWW backend
      const lww = backend as LWWBackend;
      const converted = primitiveToValue(value);
      lww.set(fieldName, converted);
    } else if (backendKind === 'yjs') {
      // Insert initial text on Yjs backend
      const yjs = backend as YjsBackend;
      if (value !== null && value !== undefined) {
        yjs.insert(fieldName, 0, String(value));
      }
    }
    // ephemeral fields are not stored in backends
  }

  // ── Transaction Forking ───────────────────────────────────────────

  /**
   * Create a transaction fork of this entity. Forks all backends for isolation.
   *
   * Rust: `pub fn snapshot(&self, trx_alive: Arc<AtomicBool>) -> Self`
   * Divergence: trx_alive is { value: boolean } instead of Arc<AtomicBool> [E8].
   */
  snapshot(trxAlive: { value: boolean }): Entity {
    // Fork all backends
    const forkedBackends = new Map<string, PropertyBackend>();
    for (const [name, backend] of this.state.backends) {
      forkedBackends.set(name, backend.fork());
    }

    return new Entity(
      this.entityId,
      this.collectionId,
      { type: 'Transacted', trxAlive, upstream: this },
      { head: this.state.head, backends: forkedBackends },
    );
  }

  // ── Event Generation (for Transaction commit) ─────────────────────

  /**
   * Generate a commit event from pending operations.
   * Returns null if no operations have been generated (no mutations).
   *
   * Rust: `pub(crate) fn generate_commit_event(&self) -> Result<Option<Event>, MutationError>`
   * Throws MutationError on failure.
   */
  generateCommitEvent(): Event | null {
    const operationMap = new Map<string, Operation[]>();

    for (const [backendName, backend] of this.state.backends) {
      try {
        const ops = backend.toOperations();
        if (ops !== null && ops.length > 0) {
          operationMap.set(backendName, ops);
        }
      } catch (e) {
        throw MutationError.general(
          e instanceof Error ? e : new Error(String(e)),
        );
      }
    }

    if (operationMap.size === 0) {
      return null; // No changes
    }

    return new Event(
      this.collectionId,
      this.entityId,
      new OperationSet(operationMap),
      this.state.head,
    );
  }

  /**
   * Update the entity's head clock after commit.
   *
   * Rust: `pub(crate) fn commit_head(&self, new_head: Clock)`
   */
  commitHead(newHead: Clock): void {
    this.state.head = newHead;
  }

  // ── Applying Operations ───────────────────────────────────────────

  /**
   * Apply operations from an event to this entity's backends.
   *
   * Rust: This is the inner loop of apply_event().
   * Throws MutationError on failure.
   */
  applyOperations(operationSet: OperationSet): void {
    for (const [backendName, ops] of operationSet.entries()) {
      const backend = this.getBackendByName(backendName);
      backend.applyOperations(ops);
    }
  }

  /**
   * Apply a full event to this entity. Simplified version without lineage comparison.
   * Sets head to the event's computed ID.
   *
   * Rust: `pub async fn apply_event<G>(&self, getter: &G, event: &Event) -> Result<bool, MutationError>`
   * Note: Full lineage comparison deferred until lineage module is ported.
   */
  applyEvent(event: Event): boolean {
    // For entity creation (empty parent), just apply directly
    if (event.isEntityCreate()) {
      if (!this.state.head.isEmpty()) {
        return false; // Already has state, skip
      }
      this.applyOperations(event.operations);
      this.state.head = Clock.fromEventId(event.id());
      this.broadcast.send();
      return true;
    }

    // Simplified lineage comparison: check if event's parent matches current head.
    // Rust: compare_unstored_event(getter, event, &head, budget) -> Ordering
    // - Descends: event.parent == head => new_head = event.id()
    // - NotDescends: event.parent != head => new_head = head.withEvent(event.id())
    //   (concurrent commit — both events should appear in head)
    let newHead: Clock;
    if (event.parent.equals(this.state.head)) {
      // Descends: this event follows directly from our head
      newHead = Clock.fromEventId(event.id());
    } else {
      // NotDescends: concurrent commit, merge into head
      // Rust: head.with_event(event.id())
      newHead = this.state.head.withEvent(event.id());
    }

    this.applyOperations(event.operations);
    this.state.head = newHead;
    this.broadcast.send();
    return true;
  }

  /**
   * Apply a complete state snapshot.
   *
   * Rust: `pub async fn apply_state<G>(&self, getter: &G, state: &State) -> Result<bool, MutationError>`
   * Simplified version without lineage comparison.
   */
  applyState(state: State): boolean {
    // Replace all backends from state buffers
    const newBackends = new Map<string, PropertyBackend>();
    for (const [name, buffer] of state.stateBuffers.entries()) {
      newBackends.set(name, backendFromString(name, buffer));
    }
    this.state.backends = newBackends;
    this.state.head = state.head;
    this.broadcast.send();
    return true;
  }

  // ── View conversion ───────────────────────────────────────────────

  /**
   * Get a typed View for this entity, if collection matches.
   *
   * Rust: `pub fn view<V: View>(&self) -> Option<V>`
   */
  view<V>(viewClass: { fromEntity(entity: Entity): V; }, expectedCollection?: CollectionId): V | null {
    if (expectedCollection !== undefined && this.collectionId !== expectedCollection) {
      return null;
    }
    return viewClass.fromEntity(this);
  }

  // ── Signal adapter ───────────────────────────────────────────────

  /** Cached Signal adapter for this entity's broadcast */
  private _signal: Signal | null = null;

  /**
   * Get a Signal adapter for this entity's broadcast.
   * Used by View getters to enable reactive tracking via CurrentObserver.
   *
   * Rust: View structs implement Subscribe which provides signal-based tracking.
   * The derive-generated View getter calls CurrentObserver::track(self) to enable
   * reactive re-evaluation when the entity changes.
   * Divergence: Entity provides signal() instead of View implementing Subscribe [E8].
   */
  signal(): Signal {
    if (!this._signal) {
      const broadcast = this.broadcast;
      this._signal = {
        listen(listener: Listener): SignalListenerGuard {
          const broadcastListener: BroadcastListener<void> = {
            type: 'NotifyOnly',
            callback: listener,
          };
          const guard = broadcast.reference().listen(broadcastListener);
          return new SignalListenerGuard(guard);
        },
        broadcastId(): BroadcastId {
          return broadcast.id();
        },
      };
    }
    return this._signal;
  }

  // ── Display ───────────────────────────────────────────────────────

  /**
   * Rust: `impl Display for Entity`
   * Output: `Entity(collection/entity_id_short clock_short)`
   */
  toString(): string {
    return `Entity(${this.collectionId}/${this.entityId.toBase64Short()} ${this.state.head.toBase64Short()})`;
  }
}

// ---------------------------------------------------------------------------
// WeakEntitySet — registry with deduplication
// ---------------------------------------------------------------------------

/**
 * Registry and factory for entities. Provides deduplication guarantees.
 *
 * Rust: `pub struct WeakEntitySet(Arc<RwLock<BTreeMap<EntityId, WeakEntity>>>)`
 * Divergence: No Arc/RwLock needed — single-threaded JS [E8].
 * Divergence: Uses WeakRef<Entity> instead of Weak<EntityInner> [E8].
 * Divergence: Uses FinalizationRegistry for auto-cleanup [E8].
 */
export class WeakEntitySet {
  private entities: Map<string, WeakRef<Entity>> = new Map();
  private registry: FinalizationRegistry<string>;

  constructor() {
    // Auto-cleanup when entities are GC'd
    this.registry = new FinalizationRegistry((key: string) => {
      this.entities.delete(key);
    });
  }

  /**
   * Get a resident entity by ID, if still alive.
   *
   * Rust: `pub fn get(&self, id: &EntityId) -> Option<Entity>`
   */
  get(id: EntityId): Entity | null {
    const key = entityIdKey(id);
    const ref_ = this.entities.get(key);
    if (!ref_) return null;
    const entity = ref_.deref();
    if (!entity) {
      this.entities.delete(key);
      return null;
    }
    return entity;
  }

  /**
   * Create a brand new entity and register it.
   *
   * Rust: `pub fn create(&self, collection: CollectionId) -> Entity`
   */
  create(collection: CollectionId): Entity {
    const id = EntityIdClass.new();
    const entity = Entity.create(id, collection);
    this.register(entity);
    return entity;
  }

  /**
   * Register an entity in the set. If an entity with the same ID already exists
   * and is still alive, the existing one is kept.
   */
  register(entity: Entity): void {
    const key = entityIdKey(entity.entityId);
    const existing = this.entities.get(key)?.deref();
    if (existing) return; // Already registered and alive
    const ref_ = new WeakRef(entity);
    this.entities.set(key, ref_);
    this.registry.register(entity, key);
  }

  /**
   * Get an entity, or create from state if not resident.
   *
   * Rust: `pub async fn with_state<R>(...) -> Result<(Option<bool>, Entity), RetrievalError>`
   * Returns [changed: boolean | null, entity: Entity].
   * changed = null if entity was not previously on node, true if state was applied, false if already existed.
   */
  withState(
    id: EntityId,
    collection: CollectionId,
    state: State,
  ): [boolean | null, Entity] {
    const existing = this.get(id);
    if (existing) {
      // Entity already resident — apply state if newer
      const changed = existing.applyState(state);
      return [changed, existing];
    }

    // Create new entity from state
    const entity = Entity.fromState(id, collection, state);
    this.register(entity);
    return [null, entity];
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Convert EntityId to a stable string key for Map lookups. */
function entityIdKey(id: EntityId): string {
  return id.toBase64();
}

/**
 * Convert a JS primitive value to a Value union.
 * Used by initializeProperty for LWW fields.
 */
function primitiveToValue(value: unknown): Value | null {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value === 'string') {
    return { type: 'String', value };
  }
  if (typeof value === 'number') {
    if (Number.isInteger(value)) {
      // Use I32 for values within signed 32-bit range, I64 for larger integers
      if (value >= -2147483648 && value <= 2147483647) {
        return { type: 'I32', value };
      }
      return { type: 'I64', value };
    }
    return { type: 'F64', value };
  }
  if (typeof value === 'boolean') {
    return { type: 'Bool', value };
  }
  // Fall back to JSON for complex types
  return { type: 'Json', value };
}
