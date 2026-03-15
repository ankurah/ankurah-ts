# `@ankurah/storage-memory` Implementation Spec

**Status**: Ready for implementation
**Dependencies**: `@ankurah/storage-common` (done), `@ankurah/proto` (done), `@ankurah/core` (Filterable, evaluatePredicate, backendFromString, Value)

## Overview

In-memory `StorageEngine` / `StorageCollection` implementation for unit testing. TS-only package (no Rust counterpart). Backed entirely by nested `Map` structures -- no persistence.

## Architecture

```
MemoryStorageEngine
  └─ collections: Map<string, MemoryStorageCollection>
                                │
                   ┌────────────┴────────────┐
                   │                         │
            states: Map<string,        events: Map<string,
              Attested<EntityState>>     Attested<Event>>
            (keyed by EntityId           (keyed by EventId
             .toBase64())                 .toBase64())
```

## File: `packages/storage-memory/src/index.ts`

Single file, ~150-200 lines. Exports two classes.

### Imports

```typescript
import type { StorageEngine, StorageCollection } from '@ankurah/core';
import type { CollectionId, EntityId, EventId, Attested, EntityState, Event } from '@ankurah/proto';
import type { Selection, Predicate } from '@ankurah/ankql';
import { RetrievalError } from '@ankurah/core';
import { evaluatePredicate, type Filterable, type Value, backendFromString } from '@ankurah/core';
import { sortItemsByOrder } from '@ankurah/storage-common';
```

**Note**: `@ankurah/core` is NOT listed in `package.json` dependencies currently. It must be added:

```json
"dependencies": {
  "@ankurah/storage-common": "workspace:*",
  "@ankurah/proto": "workspace:*",
  "@ankurah/core": "workspace:*",
  "@ankurah/ankql": "workspace:*"
}
```

### Class: `MemoryStorageEngine`

Implements `StorageEngine` from `@ankurah/core/storage.ts`.

```typescript
export class MemoryStorageEngine implements StorageEngine {
  private collections: Map<string, MemoryStorageCollection>;

  constructor();

  /** Get or create a collection. Lazy-creates on first access. */
  async collection(id: CollectionId): Promise<StorageCollection>;
}
```

#### `collection(id: CollectionId): Promise<StorageCollection>`

- Key: `id.value` (the raw string from `CollectionId`).
- If the key exists in `this.collections`, return it.
- Otherwise, create a new `MemoryStorageCollection(id)`, store it, return it.
- Never throws -- memory storage always succeeds on creation.

### Class: `MemoryStorageCollection`

Implements `StorageCollection` from `@ankurah/core/storage.ts`.

```typescript
export class MemoryStorageCollection implements StorageCollection {
  private readonly collectionId: CollectionId;
  private readonly states: Map<string, Attested<EntityState>>;
  private readonly events: Map<string, Attested<Event>>;

  constructor(collectionId: CollectionId);

  async getState(id: EntityId): Promise<Attested<EntityState>>;
  async setState(state: Attested<EntityState>): Promise<void>;
  async addEvent(event: Attested<Event>): Promise<void>;
  async getEvents(eventIds: EventId[]): Promise<Attested<Event>[]>;
  async fetchStates(selection: Selection): Promise<Attested<EntityState>[]>;
}
```

#### Data Structures

| Field | Type | Key Format | Description |
|-------|------|------------|-------------|
| `states` | `Map<string, Attested<EntityState>>` | `EntityId.toBase64()` | Current entity states |
| `events` | `Map<string, Attested<Event>>` | `EventId.toBase64()` | Event log |

Both maps use the base64 string encoding of the ID as the key. This avoids needing custom equality/hashing for the `EntityId`/`EventId` class instances.

#### Method: `getState(id: EntityId): Promise<Attested<EntityState>>`

```typescript
async getState(id: EntityId): Promise<Attested<EntityState>> {
  const key = id.toBase64();
  const state = this.states.get(key);
  if (state === undefined) {
    throw RetrievalError.entityNotFound(id);
  }
  return state;
}
```

- Lookup by `EntityId.toBase64()`.
- Throw `RetrievalError.entityNotFound(id)` if missing.
- Returns the stored `Attested<EntityState>` directly (no cloning needed -- JS objects are reference types; the caller should not mutate the returned state).

#### Method: `setState(state: Attested<EntityState>): Promise<void>`

```typescript
async setState(state: Attested<EntityState>): Promise<void> {
  const key = state.payload.entityId.toBase64();
  this.states.set(key, state);
}
```

- Upsert: always overwrites. The Rust `set_state` returns `Result<bool, MutationError>` (whether it changed), but the TS interface returns `Promise<void>`.
- Key: `state.payload.entityId.toBase64()`.

#### Method: `addEvent(event: Attested<Event>): Promise<void>`

```typescript
async addEvent(event: Attested<Event>): Promise<void> {
  const eventId = event.payload.id();
  const key = eventId.toBase64();
  // Idempotent: ON CONFLICT DO NOTHING semantics (matches Rust/SQLite behavior)
  if (!this.events.has(key)) {
    this.events.set(key, event);
  }
}
```

- Compute event ID via `event.payload.id()` (SHA-256 hash of content).
- Idempotent insert: if the event already exists, skip it (matching SQLite `ON CONFLICT DO NOTHING`).

#### Method: `getEvents(eventIds: EventId[]): Promise<Attested<Event>[]>`

```typescript
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
```

- Returns found events, silently skips missing ones (matching the Rust `get_events` behavior which returns only found events in the Vec).
- Order matches the order events are found (same as input order for present events).

#### Method: `fetchStates(selection: Selection): Promise<Attested<EntityState>[]>`

This is the most complex method. It must:
1. Iterate all stored states.
2. Filter by the selection's predicate.
3. Sort by the selection's ORDER BY clause (if any).
4. Apply the selection's LIMIT (if any).

**Strategy**: Full table scan with in-memory predicate evaluation. No query planner needed -- the planner is for backends with indexes (IndexedDB, SQLite). For memory storage, brute force scan + filter is the correct approach.

**Implementation**:

```typescript
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
      // Use sortItemsByOrder helper indirectly via comparison
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
```

### Helper: `entityStateAsFilterable`

This creates a `Filterable` adapter from an `EntityState` -- equivalent to Rust's `TemporaryEntity`. It reconstitutes property backends from the state buffers to enable field-level value access for predicate evaluation.

```typescript
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
```

**Key design decisions**:
- **Lazy backend reconstruction**: The `backendFromString(name, buffer)` call is deferred until `value()` is first called. If the predicate is `True` (match all), we avoid the cost of deserializing backends.
- **Matches Rust pattern**: The Rust SQLite `post_filter_states` creates a `TemporaryEntity` for each state, which does the same thing: `backend_from_string(name, Some(state_buffer))`.
- **Field "id" is special**: Returns `{ type: 'EntityId', value: entityState.entityId }`, matching both Rust `Filterable` impls for `Entity` and `TemporaryEntity`.

### Helper: `compareForSort`

```typescript
import { valuePartialCmp } from '@ankurah/core';
import type { OrderDirection } from '@ankurah/ankql';

function compareForSort(a: Value | null, b: Value | null, direction: OrderDirection): number {
  if (a === null && b === null) return 0;
  if (a === null) return -1;
  if (b === null) return 1;
  const cmp = valuePartialCmp(a, b);
  if (cmp === null) return 0;
  return direction === 'Asc' ? cmp : -cmp;
}
```

This is the same logic as in `storage-common/src/sorting.ts` (`compareForSort`). It can be either inlined or imported if that function is exported.

## Why Not Use the Planner?

The `Planner` from `@ankurah/storage-common` generates `Plan` objects (Index scans, TableScans with bounds, etc.) designed for backends that have indexes (IndexedDB, SQLite). For in-memory storage:

- There are no indexes to scan.
- The data set is in a flat `Map` -- the only access pattern is full iteration.
- The planner's bounds, scan directions, and order-by-spill logic are irrelevant.
- A simple `for..of` over `states.values()` + `evaluatePredicate` + `Array.sort` + `.slice` is both simpler and faster.

If this ever needs to handle large data sets, a future optimization could add in-memory indexes (e.g., `Map<fieldName, Map<fieldValue, Set<EntityId>>>`), but that is out of scope for a testing utility.

## Complete Method Signature Reference

### `MemoryStorageEngine`

| Method | Signature | Notes |
|--------|-----------|-------|
| `constructor` | `()` | No parameters |
| `collection` | `(id: CollectionId) => Promise<StorageCollection>` | Lazy-creates collections |

### `MemoryStorageCollection`

| Method | Signature | Error on |
|--------|-----------|----------|
| `getState` | `(id: EntityId) => Promise<Attested<EntityState>>` | `RetrievalError.entityNotFound(id)` if missing |
| `setState` | `(state: Attested<EntityState>) => Promise<void>` | Never throws |
| `addEvent` | `(event: Attested<Event>) => Promise<void>` | Never throws (idempotent) |
| `getEvents` | `(eventIds: EventId[]) => Promise<Attested<Event>[]>` | Never throws (skips missing) |
| `fetchStates` | `(selection: Selection) => Promise<Attested<EntityState>[]>` | Never throws (returns empty array if nothing matches) |

## Exports

```typescript
// packages/storage-memory/src/index.ts

export { MemoryStorageEngine } from './index.ts';
export { MemoryStorageCollection } from './index.ts';
```

Both the engine and the collection are exported. The collection export is useful for testing scenarios where you want direct access to the collection without going through the engine.

## Package.json Changes Required

```json
{
  "dependencies": {
    "@ankurah/storage-common": "workspace:*",
    "@ankurah/proto": "workspace:*",
    "@ankurah/core": "workspace:*",
    "@ankurah/ankql": "workspace:*"
  }
}
```

`@ankurah/core` and `@ankurah/ankql` must be added -- they are needed for:
- `@ankurah/core`: `StorageEngine`, `StorageCollection` interfaces, `RetrievalError`, `evaluatePredicate`, `Filterable`, `Value`, `backendFromString`, `valuePartialCmp`, `PropertyBackend`
- `@ankurah/ankql`: `Selection`, `OrderDirection`

## Test Plan

Tests should go in `packages/storage-memory/src/index.test.ts`:

1. **Engine creates collections on demand**: `engine.collection(id)` returns a collection; calling again with the same id returns the same instance.
2. **getState throws on missing entity**: `getState(unknownId)` throws `RetrievalError` with kind `'EntityNotFound'`.
3. **setState + getState round-trip**: Store a state, retrieve it, verify `entityId` and `state` fields match.
4. **setState overwrites**: Store a state, store a different state for the same entity, verify the latest is returned.
5. **addEvent is idempotent**: Add the same event twice, verify `getEvents` returns only one copy.
6. **getEvents returns found events**: Add 3 events, request 2 of them + 1 unknown ID, verify only the 2 known events are returned.
7. **fetchStates with True predicate**: Store 3 states, fetch with `{ type: 'True' }` predicate, verify all 3 returned.
8. **fetchStates with equality predicate**: Store states with different property values, verify filtering works.
9. **fetchStates with ORDER BY**: Verify results are sorted correctly (both ASC and DESC).
10. **fetchStates with LIMIT**: Verify result count is capped.
11. **fetchStates empty collection**: Returns empty array, no errors.

## Estimated Size

~150-200 lines of implementation code, ~150-200 lines of tests. Single agent session.
