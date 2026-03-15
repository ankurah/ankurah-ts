// MIRRORS: ankurah/core/src/resultset.rs

import type { EntityId } from '@ankurah/proto';
import {
  Broadcast,
  type BroadcastId,
  type Listener,
  ListenerGuard,
  type Signal,
} from '@ankurah/signals';

import { Disposable } from '@ankurah/std';
import { Entity } from './entity.ts';
import type { Value } from './value/index.ts';
import { extractAtPath } from './value/index.ts';
import { encodeTupleValuesWithKeySpec, type KeySpec, keySpecEquals } from './indexing/index.ts';

// ── IVec ─────────────────────────────────────────────────────────────
// Rust: `enum IVec { Small([u8; 16]), Large(Vec<u8>) }`
// Divergence: No small/large optimization needed in JS — plain Uint8Array [E8].

// ── EntityEntry ──────────────────────────────────────────────────────
// Rust: `struct EntityEntry<E: AbstractEntity> { entity: E, sort_key: Option<IVec>, dirty: bool }`
// Divergence: Specialized to Entity (no AbstractEntity generic needed) [E8].

interface EntityEntry {
  entity: Entity;
  sortKey: Uint8Array | null;
  dirty: boolean;
}

// ── ResultSetState ───────────────────────────────────────────────────
// Rust: `struct State<E: AbstractEntity> { order, index, key_spec, limit, gap_dirty }`
// Divergence: No Mutex needed — single-threaded JS [E8].
// Divergence: index uses Map<string, number> with EntityId.toBase64() keys [E8].

interface ResultSetState {
  order: EntityEntry[];
  index: Map<string, number>;
  keySpec: KeySpec | null;
  limit: number | null;
  gapDirty: boolean;
}

// ── Helpers ──────────────────────────────────────────────────────────

/** Lexicographic comparison of two Uint8Arrays. Returns -1, 0, or 1. */
function compareBytes(a: Uint8Array, b: Uint8Array): number {
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    if (a[i] < b[i]) return -1;
    if (a[i] > b[i]) return 1;
  }
  if (a.length < b.length) return -1;
  if (a.length > b.length) return 1;
  return 0;
}

/** Convert EntityId to a stable string key for Map lookups. */
function entityIdKey(id: EntityId): string {
  return id.toBase64();
}

/** Compare two EntityIds by their byte representation. */
function compareEntityIds(a: EntityId, b: EntityId): number {
  return compareBytes(a.toBytes(), b.toBytes());
}

/**
 * Compare two EntityEntry values for sorting.
 * Sorts by sortKey first, then by entityId for tie-breaking.
 * Rust: inline closure in binary_search_by and sort_by.
 */
function compareEntries(a: EntityEntry, b: EntityEntry): number {
  if (a.sortKey !== null && b.sortKey !== null) {
    const keyCmp = compareBytes(a.sortKey, b.sortKey);
    if (keyCmp !== 0) return keyCmp;
    return compareEntityIds(a.entity.id(), b.entity.id());
  }
  if (a.sortKey !== null && b.sortKey === null) {
    return -1; // Keyed entries sort before unkeyed
  }
  if (a.sortKey === null && b.sortKey !== null) {
    return 1; // Unkeyed entries sort after keyed
  }
  // Both unkeyed - sort by entity ID
  return compareEntityIds(a.entity.id(), b.entity.id());
}

/**
 * Rebuild index from a given position.
 * Rust: `fn fix_from<E: AbstractEntity>(st: &mut State<E>, start: usize)`
 */
function fixFrom(state: ResultSetState, start: number): void {
  for (let i = start; i < state.order.length; i++) {
    const id = entityIdKey(state.order[i].entity.id());
    state.index.set(id, i);
  }
}

/**
 * Compute sort key for an entity using the given key spec.
 * Rust: `fn compute_sort_key(entity: &E, key_spec: &KeySpec) -> IVec`
 */
function computeSortKey(entity: Entity, keySpec: KeySpec): Uint8Array {
  const values: Value[] = [];

  // Extract values for each key part
  for (const keypart of keySpec.keyparts) {
    const rawValue = entity.getPropertyValue(keypart.column);
    if (rawValue === null) {
      // Skip this entity for now if any field is NULL
      return new Uint8Array(0); // Empty key sorts first
    }
    // Handle sub_path extraction
    let value: Value | null;
    if (keypart.subPath !== null) {
      value = extractAtPath(rawValue, keypart.subPath);
    } else {
      value = rawValue;
    }
    if (value === null) {
      return new Uint8Array(0); // Empty key sorts first
    }
    values.push(value);
  }

  // Encode the tuple
  return encodeTupleValuesWithKeySpec(values, keySpec);
}

/**
 * Binary search for the correct insertion position in a sorted array.
 * Returns the index where the entry should be inserted.
 */
function binarySearchInsertPos(order: EntityEntry[], entry: EntityEntry): number {
  let lo = 0;
  let hi = order.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    const cmp = compareEntries(order[mid], entry);
    if (cmp < 0) {
      lo = mid + 1;
    } else if (cmp > 0) {
      hi = mid;
    } else {
      // Found exact match (same sort key and entity ID) — insert here
      return mid;
    }
  }
  return lo;
}

// ── ResultSetWrite ───────────────────────────────────────────────────
// Rust: `pub struct ResultSetWrite<'a, E: AbstractEntity = Entity>`
// Divergence: No lifetime parameter — JS has no lifetimes [E8].
// Divergence: No MutexGuard — single-threaded JS [E8].
// Divergence: impl Drop -> extends Disposable [E11].

export class ResultSetWrite extends Disposable {
  private resultset: EntityResultSet;
  private changed: boolean;
  private state: ResultSetState;

  /** @internal */
  constructor(resultset: EntityResultSet, state: ResultSetState) {
    super('ResultSetWrite', 'fatal');
    this.resultset = resultset;
    this.changed = false;
    this.state = state;
  }

  /**
   * Add an entity to the result set.
   * Rust: `pub fn add(&mut self, entity: E) -> bool`
   */
  add(entity: Entity): boolean {
    const id = entityIdKey(entity.id());
    if (this.state.index.has(id)) {
      return false; // Already present
    }

    // Compute sort key if ordering is configured
    const sortKey = this.state.keySpec !== null
      ? computeSortKey(entity, this.state.keySpec)
      : null;

    const entry: EntityEntry = { entity, sortKey, dirty: false };

    // Insert in correct position (always sort by entity ID, with optional key spec first)
    const pos = binarySearchInsertPos(this.state.order, entry);

    this.state.order.splice(pos, 0, entry);
    this.state.index.set(id, pos);

    // Fix indices for all entries after the insertion point
    for (let i = pos + 1; i < this.state.order.length; i++) {
      const entryId = entityIdKey(this.state.order[i].entity.id());
      this.state.index.set(entryId, i);
    }

    // Apply limit if configured
    if (this.state.limit !== null) {
      if (this.state.order.length > this.state.limit) {
        // Remove the last entry (beyond limit)
        const removedEntry = this.state.order.pop();
        if (removedEntry) {
          const removedId = entityIdKey(removedEntry.entity.id());
          this.state.index.delete(removedId);
        }
      }
    }

    this.changed = true;
    return true;
  }

  /**
   * Remove an entity from the result set.
   * Rust: `pub fn remove(&mut self, id: proto::EntityId) -> bool`
   */
  remove(id: EntityId): boolean {
    const key = entityIdKey(id);
    const idx = this.state.index.get(key);
    if (idx === undefined) {
      return false;
    }

    // Check if we were at limit before removal
    if (this.state.limit !== null && this.state.order.length === this.state.limit) {
      this.state.gapDirty = true;
    }

    this.state.index.delete(key);
    this.state.order.splice(idx, 1);
    if (idx < this.state.order.length) {
      fixFrom(this.state, idx);
    }

    this.changed = true;
    return true;
  }

  /**
   * Check if an entity exists.
   * Rust: `pub fn contains(&self, id: &proto::EntityId) -> bool`
   */
  contains(id: EntityId): boolean {
    return this.state.index.has(entityIdKey(id));
  }

  /**
   * Iterate over all entities.
   * Returns an array of [entityId, entity] pairs.
   * Rust: `pub fn iter_entities(&self) -> impl Iterator<Item = (proto::EntityId, &E)>`
   * Divergence: Returns array instead of iterator [E8].
   */
  iterEntities(): Array<[EntityId, Entity]> {
    return this.state.order.map((entry) => [entry.entity.id(), entry.entity]);
  }

  /**
   * Mark all entities as dirty for re-evaluation.
   * Rust: `pub fn mark_all_dirty(&mut self)`
   */
  markAllDirty(): void {
    for (const entry of this.state.order) {
      entry.dirty = true;
    }
    this.changed = true;
  }

  /**
   * Retain only dirty entities that pass the closure, removing those that don't.
   * Rust: `pub fn retain_dirty<F>(&mut self, should_retain: F) -> Vec<proto::EntityId>`
   */
  retainDirty(shouldRetain: (entity: Entity) => boolean): EntityId[] {
    const removedIds: EntityId[] = [];
    let i = 0;

    // Check if we were at limit before any removals
    const wasAtLimit = this.state.limit !== null && this.state.order.length === this.state.limit;

    while (i < this.state.order.length) {
      if (this.state.order[i].dirty) {
        const shouldKeep = shouldRetain(this.state.order[i].entity);
        if (shouldKeep) {
          // Entity should be retained - recompute sort key and mark clean
          if (this.state.keySpec !== null) {
            this.state.order[i].sortKey = computeSortKey(this.state.order[i].entity, this.state.keySpec);
          }
          this.state.order[i].dirty = false;
          i++;
        } else {
          // Entity should be removed
          const removedEntry = this.state.order.splice(i, 1)[0];
          const removedId = removedEntry.entity.id();
          this.state.index.delete(entityIdKey(removedId));
          removedIds.push(removedId);
          // Don't increment i since we removed an element
        }
      } else {
        i++;
      }
    }

    // Fix indices after removals (no re-sorting needed)
    this.state.index.clear();
    for (let j = 0; j < this.state.order.length; j++) {
      this.state.index.set(entityIdKey(this.state.order[j].entity.id()), j);
    }

    if (removedIds.length > 0) {
      this.changed = true;

      // Set gapDirty if we went from LIMIT to < LIMIT
      if (!this.state.gapDirty && wasAtLimit && this.state.limit !== null && this.state.order.length < this.state.limit) {
        this.state.gapDirty = true;
      }
    }

    return removedIds;
  }

  /**
   * Replace all entities in the result set with proper sorting.
   * Rust: `pub fn replace_all(&mut self, entities: Vec<E>)`
   */
  replaceAll(entities: Entity[]): void {
    // Clear existing data
    this.state.order.length = 0;
    this.state.index.clear();

    // Add all entities with proper sorting
    for (const entity of entities) {
      // Compute sort key if ordering is configured
      const sortKey = this.state.keySpec !== null
        ? computeSortKey(entity, this.state.keySpec)
        : null;

      const entry: EntityEntry = { entity, sortKey, dirty: false };
      this.state.order.push(entry);
    }

    // Sort all entries
    if (this.state.keySpec !== null) {
      this.state.order.sort(compareEntries);
    } else {
      // Sort by entity ID only if no key spec
      this.state.order.sort((a, b) => compareEntityIds(a.entity.id(), b.entity.id()));
    }

    // Apply limit if configured
    if (this.state.limit !== null) {
      if (this.state.order.length > this.state.limit) {
        this.state.order.length = this.state.limit;
      }
    }

    // Rebuild index
    for (let i = 0; i < this.state.order.length; i++) {
      this.state.index.set(entityIdKey(this.state.order[i].entity.id()), i);
    }

    this.changed = true;
  }

  /**
   * Set the loaded flag as part of this write transaction.
   * Rust: `pub fn set_loaded(&mut self, loaded: bool)`
   */
  setLoaded(loaded: boolean): void {
    this.resultset._setLoadedDirect(loaded);
    this.changed = true; // Ensure we broadcast on done()
  }

  /**
   * Finish the write operation — broadcasts if changed.
   * Mirrors Rust Drop impl for ResultSetWrite [E11].
   */
  protected onDispose(): void {
    if (this.changed) {
      this.resultset._broadcast();
    }
  }

  /**
   * Compatibility alias — prefer `using` or explicit `dispose()`.
   * @deprecated Use `dispose()` or `using` instead.
   */
  done(): void {
    this.dispose();
  }
}

// ── ResultSetRead ────────────────────────────────────────────────────
// Rust: `pub struct ResultSetRead<'a, E: AbstractEntity = Entity>`
// Divergence: No lifetime parameter or MutexGuard — single-threaded JS [E8].

export class ResultSetRead {
  private state: ResultSetState;

  /** @internal */
  constructor(state: ResultSetState) {
    this.state = state;
  }

  /**
   * Check if an entity exists.
   * Rust: `pub fn contains(&self, id: &proto::EntityId) -> bool`
   */
  contains(id: EntityId): boolean {
    return this.state.index.has(entityIdKey(id));
  }

  /**
   * Iterate over all entities.
   * Returns an array of [entityId, entity] pairs.
   * Rust: `pub fn iter_entities(&self) -> impl Iterator<Item = (proto::EntityId, &E)>`
   * Divergence: Returns array instead of iterator [E8].
   */
  iterEntities(): Array<[EntityId, Entity]> {
    return this.state.order.map((entry) => [entry.entity.id(), entry.entity]);
  }

  /**
   * Get the number of entities.
   * Rust: `pub fn len(&self) -> usize`
   */
  len(): number {
    return this.state.order.length;
  }

  /**
   * Check if the result set is empty.
   * Rust: `pub fn is_empty(&self) -> bool`
   */
  isEmpty(): boolean {
    return this.state.order.length === 0;
  }
}

// ── EntityResultSet ──────────────────────────────────────────────────
// Rust: `pub struct EntityResultSet<E: AbstractEntity = Entity>(Arc<Inner<E>>)`
// Divergence: No Arc — plain class instance (JS single-threaded, GC handles memory) [E8].
// Divergence: No Mutex on state — single-threaded JS [E8].
// Divergence: No AtomicBool — plain boolean [E8].
// Divergence: Specialized to Entity (no AbstractEntity generic needed) [E8].

export class EntityResultSet implements Signal {
  private state: ResultSetState;
  private loaded: boolean;
  private _broadcastInner: Broadcast;

  private constructor(state: ResultSetState, loaded: boolean) {
    this.state = state;
    this.loaded = loaded;
    this._broadcastInner = new Broadcast();
  }

  // ── Static constructors ─────────────────────────────────────────

  /** Rust: `pub fn from_vec(entities: Vec<E>, loaded: bool) -> Self` */
  static fromVec(entities: Entity[], loaded: boolean): EntityResultSet {
    const index = new Map<string, number>();
    const order: EntityEntry[] = [];

    for (let i = 0; i < entities.length; i++) {
      const entity = entities[i];
      index.set(entityIdKey(entity.id()), i);
      order.push({ entity, sortKey: null, dirty: false });
    }

    const state: ResultSetState = { order, index, keySpec: null, limit: null, gapDirty: false };
    return new EntityResultSet(state, loaded);
  }

  /** Rust: `pub fn empty() -> Self` */
  static empty(): EntityResultSet {
    const state: ResultSetState = { order: [], index: new Map(), keySpec: null, limit: null, gapDirty: false };
    return new EntityResultSet(state, false);
  }

  /** Rust: `pub fn single(entity: E) -> Self` */
  static single(entity: Entity): EntityResultSet {
    const entry: EntityEntry = { entity, sortKey: null, dirty: false };
    const index = new Map<string, number>();
    index.set(entityIdKey(entity.id()), 0);
    const state: ResultSetState = { order: [entry], index, keySpec: null, limit: null, gapDirty: false };
    return new EntityResultSet(state, false);
  }

  // ── Guards ──────────────────────────────────────────────────────

  /**
   * Begin a write operation for atomic changes to the resultset.
   * All mutations happen through the returned write guard.
   * A single notification is sent when done() is called (if changes were made).
   * Rust: `pub fn write(&self) -> ResultSetWrite<'_, E>`
   * Divergence: No MutexGuard — single-threaded JS [E8].
   */
  write(): ResultSetWrite {
    return new ResultSetWrite(this, this.state);
  }

  /**
   * Get a read guard for consistent read-only access to the resultset.
   * Rust: `pub fn read(&self) -> ResultSetRead<'_, E>`
   * Divergence: No MutexGuard — single-threaded JS [E8].
   */
  read(): ResultSetRead {
    return new ResultSetRead(this.state);
  }

  // ── Direct methods ─────────────────────────────────────────────

  /** Rust: `pub fn set_loaded(&self, loaded: bool)` */
  setLoaded(loaded: boolean): void {
    this.loaded = loaded;
    this._broadcastInner.send();
  }

  /** Rust: `pub fn is_loaded(&self) -> bool` */
  isLoaded(): boolean {
    // TODO: CurrentObserver::track() when observer system is ported
    return this.loaded;
  }

  /** Rust: `pub fn clear(&self)` */
  clear(): void {
    this.state.order.length = 0;
    this.state.index.clear();
    this._broadcastInner.send();
  }

  /**
   * Get an array of entity IDs.
   * Rust: `pub fn keys(&self) -> EntityResultSetKeyIterator`
   * Divergence: Returns array instead of custom iterator [E8].
   */
  keys(): EntityId[] {
    // TODO: CurrentObserver::track() when observer system is ported
    return this.state.order.map((e) => e.entity.id());
  }

  /**
   * Check if an entity with the given ID exists.
   * Rust: `pub fn contains_key(&self, id: &proto::EntityId) -> bool`
   */
  containsKey(id: EntityId): boolean {
    // TODO: CurrentObserver::track() when observer system is ported
    return this.state.index.has(entityIdKey(id));
  }

  /**
   * Get an entity by ID.
   * Rust: `pub fn by_id(&self, id: &proto::EntityId) -> Option<E>`
   */
  byId(id: EntityId): Entity | null {
    // TODO: CurrentObserver::track() when observer system is ported
    const idx = this.state.index.get(entityIdKey(id));
    if (idx === undefined) return null;
    return this.state.order[idx].entity;
  }

  /**
   * Get the number of entities.
   * Rust: `pub fn len(&self) -> usize`
   */
  len(): number {
    // TODO: CurrentObserver::track() when observer system is ported
    return this.state.order.length;
  }

  // ── Internal methods ───────────────────────────────────────────

  /**
   * Check if this result set needs gap filling.
   * Rust: `pub(crate) fn is_gap_dirty(&self) -> bool`
   */
  isGapDirty(): boolean {
    return this.state.gapDirty;
  }

  /**
   * Clear the gap_dirty flag (called after gap filling is complete).
   * Rust: `pub(crate) fn clear_gap_dirty(&self)`
   */
  clearGapDirty(): void {
    this.state.gapDirty = false;
  }

  /**
   * Get the current limit for this result set.
   * Rust: `pub fn get_limit(&self) -> Option<usize>`
   */
  getLimit(): number | null {
    return this.state.limit;
  }

  /**
   * Get the last entity for gap filling continuation.
   * Rust: `pub(crate) fn last_entity(&self) -> Option<E>`
   */
  lastEntity(): Entity | null {
    if (this.state.order.length === 0) return null;
    return this.state.order[this.state.order.length - 1].entity;
  }

  // ── Config ─────────────────────────────────────────────────────

  /**
   * Configure ordering for this result set.
   * Rust: `pub(crate) fn order_by(&self, key_spec: Option<KeySpec>)`
   */
  orderBy(keySpec: KeySpec | null): void {
    // Check if the key spec actually changed
    if (this.state.keySpec === null && keySpec === null) return;
    if (this.state.keySpec !== null && keySpec !== null && keySpecEquals(this.state.keySpec, keySpec)) return;

    this.state.keySpec = keySpec;

    // Recompute sort keys for all entries
    for (const entry of this.state.order) {
      if (keySpec !== null) {
        entry.sortKey = computeSortKey(entry.entity, keySpec);
      } else {
        entry.sortKey = null; // No ORDER BY, sort by entity ID only
      }
    }

    // Sort by the new keys
    if (keySpec !== null) {
      this.state.order.sort(compareEntries);
    } else {
      this.state.order.sort((a, b) => compareEntityIds(a.entity.id(), b.entity.id()));
    }

    // Rebuild index after sorting
    this.state.index.clear();
    for (let i = 0; i < this.state.order.length; i++) {
      this.state.index.set(entityIdKey(this.state.order[i].entity.id()), i);
    }

    this._broadcastInner.send();
  }

  /**
   * Set the limit for this result set.
   * Rust: `pub(crate) fn limit(&self, limit: Option<usize>)`
   */
  setLimit(limit: number | null): void {
    // Check if the limit actually changed
    if (this.state.limit === limit) return;

    this.state.limit = limit;

    // Apply the new limit by truncating if necessary
    let entitiesRemoved = false;
    if (limit !== null) {
      if (this.state.order.length > limit) {
        // Remove entries beyond the limit from the index
        for (let i = limit; i < this.state.order.length; i++) {
          this.state.index.delete(entityIdKey(this.state.order[i].entity.id()));
        }
        this.state.order.length = limit;
        entitiesRemoved = true;
      }
    }

    // Only broadcast if entities were actually removed
    if (entitiesRemoved) {
      this._broadcastInner.send();
    }
  }

  // ── Signal impl ────────────────────────────────────────────────
  // Rust: `impl<E: AbstractEntity> Signal for EntityResultSet<E>`

  /** Rust: `fn listen(&self, listener: Listener) -> ListenerGuard` */
  listen(listener: Listener): ListenerGuard {
    return new ListenerGuard(
      this._broadcastInner.reference().listen({ type: 'NotifyOnly', callback: listener }),
    );
  }

  /** Rust: `fn broadcast_id(&self) -> BroadcastId` */
  broadcastId(): BroadcastId {
    return this._broadcastInner.id();
  }

  // ── Internal helpers (used by ResultSetWrite) ──────────────────

  /** @internal — Set loaded flag without broadcasting. Used by ResultSetWrite.setLoaded(). */
  _setLoadedDirect(loaded: boolean): void {
    this.loaded = loaded;
  }

  /** @internal — Send broadcast. Used by ResultSetWrite.done(). */
  _broadcast(): void {
    this._broadcastInner.send();
  }
}
