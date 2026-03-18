// TS-ONLY: In-memory StorageEngine implementation for testing
//
// @ankurah/storage-memory -- StorageEngine/StorageCollection backed by plain Maps.
// No persistence -- intended for unit tests only.

import type { StorageEngine, StorageCollection } from '@ankurah/core';
import type { CollectionId, EntityId, EventId, Attested, EntityState, Event } from '@ankurah/proto';
import type { Selection, OrderDirection } from '@ankurah/ankql';
import { RetrievalError } from '@ankurah/core';
import { evaluatePredicate, type Filterable, type Value, backendFromString, valuePartialCmp } from '@ankurah/core';
import type { PropertyBackend } from '@ankurah/core';

// ---------------------------------------------------------------------------
// MemoryStorageEngine
// ---------------------------------------------------------------------------

/**
 * In-memory StorageEngine. Lazy-creates collections on first access.
 * All data lives in JS Maps -- nothing is persisted.
 */
export class MemoryStorageEngine implements StorageEngine {
  private collections: Map<string, MemoryStorageCollection>;

  constructor() {
    this.collections = new Map();
  }

  /** Get or create a collection. Lazy-creates on first access. */
  async collection(id: CollectionId): Promise<StorageCollection> {
    const key = id.value;
    let coll = this.collections.get(key);
    if (coll === undefined) {
      coll = new MemoryStorageCollection(id);
      this.collections.set(key, coll);
    }
    return coll;
  }

  /** Delete all collections and their data. */
  async deleteAllCollections(): Promise<boolean> {
    const hadData = this.collections.size > 0;
    this.collections.clear();
    return hadData;
  }

  /** List all collection names in the storage engine (Rust: SledStorageEngine::list_collections). */
  listCollections(): string[] {
    return Array.from(this.collections.keys()).sort();
  }
}

// ---------------------------------------------------------------------------
// MemoryStorageCollection
// ---------------------------------------------------------------------------

/**
 * In-memory StorageCollection. Stores entity states and events in Maps,
 * keyed by their base64-encoded IDs.
 */
export class MemoryStorageCollection implements StorageCollection {
  private readonly collectionId: CollectionId;
  private readonly states: Map<string, Attested<EntityState>>;
  private readonly events: Map<string, Attested<Event>>;

  constructor(collectionId: CollectionId) {
    this.collectionId = collectionId;
    this.states = new Map();
    this.events = new Map();
  }

  async getState(id: EntityId): Promise<Attested<EntityState>> {
    const key = id.toBase64();
    const state = this.states.get(key);
    if (state === undefined) {
      throw RetrievalError.entityNotFound(id);
    }
    return state;
  }

  async setState(state: Attested<EntityState>): Promise<boolean> {
    const key = state.payload.entityId.toBase64();
    const existed = this.states.has(key);
    this.states.set(key, state);
    return !existed;
  }

  async addEvent(event: Attested<Event>): Promise<boolean> {
    const eventId = event.payload.id();
    const key = eventId.toBase64();
    // Idempotent: ON CONFLICT DO NOTHING semantics (matches Rust/SQLite behavior)
    if (!this.events.has(key)) {
      this.events.set(key, event);
      return true;
    }
    return false;
  }

  async getEvents(eventIds: EventId[]): Promise<Attested<Event>[]> {
    const results: Attested<Event>[] = [];
    for (const id of eventIds) {
      const event = this.events.get(id.toBase64());
      if (event !== undefined) {
        results.push(event);
      }
    }
    return results;
  }

  async dumpEntityEvents(id: EntityId): Promise<Attested<Event>[]> {
    const results: Attested<Event>[] = [];
    for (const event of this.events.values()) {
      if (event.payload.entityId.equals(id)) {
        results.push(event);
      }
    }
    return results;
  }

  async fetchStates(selection: Selection): Promise<Attested<EntityState>[]> {
    let results: Attested<EntityState>[] = [];

    // 1. Filter: iterate all states, evaluate predicate on each
    for (const attested of this.states.values()) {
      const filterable = entityStateAsFilterable(attested.payload, this.collectionId);
      if (evaluatePredicate(filterable, selection.predicate)) {
        results.push(attested);
      }
    }

    // 2. Sort: if ORDER BY is specified, sort in-place
    if (selection.orderBy !== null && selection.orderBy.length > 0) {
      const orderBy = selection.orderBy;
      results.sort((a, b) => {
        const fa = entityStateAsFilterable(a.payload, this.collectionId);
        const fb = entityStateAsFilterable(b.payload, this.collectionId);
        for (const item of orderBy) {
          const propName = item.path.property();
          const aVal = fa.value(propName);
          const bVal = fb.value(propName);
          const cmp = compareForSort(aVal, bVal, item.direction);
          if (cmp !== 0) return cmp;
        }
        return 0;
      });
    }

    // 3. Limit: truncate if limit specified
    if (selection.limit !== null && selection.limit > 0) {
      results = results.slice(0, selection.limit);
    }

    return results;
  }
}

// ---------------------------------------------------------------------------
// Helper: entityStateAsFilterable
// ---------------------------------------------------------------------------

/**
 * Creates a Filterable adapter from an EntityState.
 * Equivalent to Rust's TemporaryEntity -- reconstitutes property backends
 * from state buffers to enable field-level value access for predicate evaluation.
 */
function entityStateAsFilterable(
  entityState: EntityState,
  collectionId: CollectionId,
): Filterable {
  // Lazily reconstruct backends from state buffers
  let backends: Map<string, PropertyBackend> | null = null;

  function getBackends(): Map<string, PropertyBackend> {
    if (backends === null) {
      backends = new Map();
      for (const [name, buffer] of entityState.state.stateBuffers) {
        backends.set(name, backendFromString(name, buffer));
      }
    }
    return backends;
  }

  return {
    collection(): string {
      return collectionId.value;
    },
    value(name: string): Value | null {
      if (name === 'id') {
        return { type: 'EntityId', value: entityState.entityId };
      }
      for (const backend of getBackends().values()) {
        const v = backend.propertyValue(name);
        if (v !== null) return v;
      }
      return null;
    },
  };
}

// ---------------------------------------------------------------------------
// Helper: compareForSort
// ---------------------------------------------------------------------------

/**
 * Compare two optional Values for sorting, respecting direction.
 * Null sorts before non-null (NULLS FIRST semantics).
 */
function compareForSort(a: Value | null, b: Value | null, direction: OrderDirection): number {
  if (a === null && b === null) return 0;
  if (a === null) return -1;
  if (b === null) return 1;
  const cmp = valuePartialCmp(a, b);
  if (cmp === null) return 0;
  return direction.is('Asc') ? cmp : -cmp;
}
