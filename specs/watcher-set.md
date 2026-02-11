# WatcherSet Port Specification

> **Source:** `ankurah/core/src/reactor/watcherset.rs`
> **Target:** `ankurah-ts/packages/core/src/reactor/watcher-set.ts`
> **MIRRORS:** `ankurah/core/src/reactor/watcherset.rs`

---

## 1. Overview

`WatcherSet` is the central routing table that maps entity changes to interested subscriptions. It maintains three registries:

1. **Index watchers** -- per-(collection, property-path) `ComparisonIndex` instances that efficiently match field-value comparisons from predicate clauses.
2. **Wildcard watchers** -- per-collection sets of watchers that match *any* entity change in a collection (i.e. predicates that resolve to `Predicate::True`).
3. **Entity watchers** -- per-entity-id sets of watcher IDs. Once an entity has been confirmed to match a predicate (or has been explicitly subscribed), a per-entity watcher is registered so that future changes to that same entity are routed without re-evaluating comparison indexes.

The three registries work together in `accumulateInterestedWatchers()` to fan-out a single entity change to all possibly-interested subscriptions, producing a `CandidateChanges` per subscription.

---

## 2. File Header & Imports

```ts
// MIRRORS: ankurah/core/src/reactor/watcherset.rs

import type { CollectionId, EntityId, QueryId } from '@ankurah/proto';
import type { Predicate, ComparisonOperator, Literal, PathExpr, Expr } from '@ankurah/ankql';
import { ComparisonIndex } from './comparison-index.ts';
import { PropertyPath } from './property-path.ts';
import { CandidateChanges } from './candidate-changes.ts';
import type { Entity } from '../entity.ts';
import type { Value } from '../value/index.ts';
```

---

## 3. Prerequisite Type: `ReactorSubscriptionId`

Rust defines this in `reactor/subscription.rs` as a newtype around `Ulid`. It is not yet present in the TS codebase. It must be defined before or alongside `WatcherSet`.

```ts
/**
 * Unique identifier for a reactor subscription, local to a single reactor/node.
 * Cannot be transported across nodes. Mirrors Rust ReactorSubscriptionId(Ulid).
 *
 * Uses a ULID string as its identity for Map-key usage (JS lacks structural equality on objects).
 */
export class ReactorSubscriptionId {
  private readonly ulid: string; // 26-char Crockford Base32 ULID string

  private constructor(ulid: string) {
    this.ulid = ulid;
  }

  static new(): ReactorSubscriptionId {
    // Generate a new ULID (use the same ULID generation utility as the proto package)
    return new ReactorSubscriptionId(generateUlid());
  }

  /** String key for use in Maps. */
  toKey(): string {
    return this.ulid;
  }

  toString(): string {
    return `RS-${this.ulid}`;
  }

  equals(other: ReactorSubscriptionId): boolean {
    return this.ulid === other.ulid;
  }
}
```

**Placement decision:** Define in `reactor/subscription.ts` or a new `reactor/types.ts` file -- whichever the project structure prefers. It is imported by `watcher-set.ts`.

---

## 4. Supporting Types Defined in `watcher-set.ts`

### 4.1 `WatcherOp`

```ts
/**
 * Whether a watcher is being added or removed.
 * Rust: `pub enum WatcherOp { Add, Remove }`
 */
export type WatcherOp = 'Add' | 'Remove';
```

### 4.2 `EntityWatcherId`

```ts
/**
 * Identifies how an entity is being watched -- either by a predicate query
 * or by an explicit entity subscription.
 *
 * Rust: `enum EntityWatcherId { Predicate(ReactorSubscriptionId, QueryId), Subscription(ReactorSubscriptionId) }`
 *
 * Divergence: Rust derives Hash/Eq/Ord automatically. TS needs a stable string
 * key for use in Set<string> since JS Sets use reference equality.
 */
export type EntityWatcherId =
  | { type: 'Predicate'; subscriptionId: ReactorSubscriptionId; queryId: QueryId }
  | { type: 'Subscription'; subscriptionId: ReactorSubscriptionId };

/** Extract the subscription ID from either variant. */
export function entityWatcherSubscriptionId(ew: EntityWatcherId): ReactorSubscriptionId {
  return ew.subscriptionId;
}

/**
 * Produce a stable string key for an EntityWatcherId, suitable for use
 * as a key in a Set<string> or Map<string, ...>.
 *
 * Rust gets this for free via Hash+Eq derive; JS needs explicit serialization.
 */
export function entityWatcherIdKey(ew: EntityWatcherId): string {
  if (ew.type === 'Predicate') {
    return `P:${ew.subscriptionId.toKey()}:${ew.queryId.toUlidString()}`;
  }
  return `S:${ew.subscriptionId.toKey()}`;
}
```

### 4.3 `WatcherChange`

```ts
/**
 * Represents a deferred mutation to entity watchers that should be applied
 * after evaluate_changes completes (to avoid holding locks across async work).
 *
 * Rust: `pub enum WatcherChange { Add { ... }, Remove { ... } }`
 */
export type WatcherChange =
  | { type: 'Add'; entityId: EntityId; subscriptionId: ReactorSubscriptionId; queryId: QueryId }
  | { type: 'Remove'; entityId: EntityId; subscriptionId: ReactorSubscriptionId; queryId: QueryId };

/** Factory: create an Add watcher change. */
export function watcherChangeAdd(
  entityId: EntityId,
  subscriptionId: ReactorSubscriptionId,
  queryId: QueryId,
): WatcherChange {
  return { type: 'Add', entityId, subscriptionId, queryId };
}

/** Factory: create a Remove watcher change. */
export function watcherChangeRemove(
  entityId: EntityId,
  subscriptionId: ReactorSubscriptionId,
  queryId: QueryId,
): WatcherChange {
  return { type: 'Remove', entityId, subscriptionId, queryId };
}
```

### 4.4 Composite Key Helper Type

Both `index_watchers` and `wildcard_watchers` in Rust use composite HashMap keys:
- `(CollectionId, PropertyPath)` for index watchers
- `CollectionId` for wildcard watchers

Since JS Maps use reference equality, we need string keys.

```ts
/** Build a composite map key from collection ID + property path. */
function indexWatcherKey(collectionId: CollectionId, propertyPath: PropertyPath): string {
  return `${collectionId.toString()}::${propertyPath.toString()}`;
}
```

### 4.5 Watcher ID Tuple Type

In Rust, the index watchers and wildcard watchers store `(ReactorSubscriptionId, QueryId)` tuples as their subscriber type parameter `T` in `ComparisonIndex<T>`.

```ts
/**
 * The subscriber identity stored inside ComparisonIndex and wildcard sets.
 * Mirrors Rust tuple `(ReactorSubscriptionId, proto::QueryId)`.
 */
export interface WatcherIdPair {
  subscriptionId: ReactorSubscriptionId;
  queryId: QueryId;
}

/**
 * Produce a stable string key for a WatcherIdPair, for use with Set<string>.
 */
export function watcherIdPairKey(pair: WatcherIdPair): string {
  return `${pair.subscriptionId.toKey()}:${pair.queryId.toUlidString()}`;
}
```

**Important:** `ComparisonIndex<T>` in the existing TS code uses `T` with `indexOf()` for removal, which relies on reference equality. For `WatcherIdPair`, we need value equality. Two options:

- **Option A (recommended):** Change the `ComparisonIndex<T>` subscriber type to `string` (the serialized key), and maintain a side-map from key back to the pair when iterating results. This avoids modifying `ComparisonIndex`.
- **Option B:** Modify `ComparisonIndex` to accept a custom equality/key function.
- **Option C:** Use the string key directly as `T = string` in `ComparisonIndex<string>` and store separate lookup maps.

**Recommendation:** Use `ComparisonIndex<string>` where the string is `watcherIdPairKey(pair)`, and maintain a `Map<string, WatcherIdPair>` for reverse lookup when iterating results. This is noted in the method signatures below.

---

## 5. `WatcherSet` Class

### 5.1 Fields

```ts
export class WatcherSet {
  /**
   * Per (collection, property-path) comparison indexes.
   * Rust: HashMap<(CollectionId, PropertyPath), ComparisonIndex<(ReactorSubscriptionId, QueryId)>>
   *
   * Key: composite string from `indexWatcherKey()`.
   * Value: ComparisonIndex<string> where strings are watcherIdPairKey serializations.
   *
   * Divergence from Rust:
   * - Rust HashMap with tuple key -> JS Map with string key.
   * - Rust ComparisonIndex<(SubId, QueryId)> -> TS ComparisonIndex<string> with a reverse lookup map.
   */
  private indexWatchers: Map<string, ComparisonIndex<string>> = new Map();

  /**
   * Reverse lookup: watcherIdPairKey string -> WatcherIdPair.
   * Shared across all ComparisonIndex instances so that find_matching results
   * can be resolved back to (subscriptionId, queryId).
   */
  private watcherIdLookup: Map<string, WatcherIdPair> = new Map();

  /**
   * Per-collection sets of watchers that match ANY entity change.
   * Rust: HashMap<CollectionId, HashSet<(ReactorSubscriptionId, QueryId)>>
   *
   * Key: collectionId.toString()
   * Value: Map<string, WatcherIdPair> keyed by watcherIdPairKey.
   *
   * Divergence: Rust HashSet<tuple> -> JS Map<string, WatcherIdPair> for value equality.
   */
  private wildcardWatchers: Map<string, Map<string, WatcherIdPair>> = new Map();

  /**
   * Per-entity watcher registrations.
   * Rust: HashMap<EntityId, HashSet<EntityWatcherId>>
   *
   * Key: entityId.toBase64() (stable string for value equality)
   * Value: Map<string, EntityWatcherId> keyed by entityWatcherIdKey.
   *
   * Divergence: Rust HashSet<EntityWatcherId> -> JS Map<string, EntityWatcherId> for value equality.
   */
  private entityWatchers: Map<string, Map<string, EntityWatcherId>> = new Map();
}
```

**Concurrency simplification:** Rust wraps `WatcherSet` in `Arc<Mutex<WatcherSet>>` for thread safety. In single-threaded JS, `WatcherSet` is a plain object with no locking. The `Reactor` that owns it simply holds a direct reference.

### 5.2 Constructor

```ts
constructor() {
  // All maps initialized to empty in field declarations.
}
```

---

## 6. Methods

### 6.1 `recursePredicateWatchers()`

Walks an AnkQL predicate tree and adds/removes watcher entries in the index and wildcard registries.

```ts
/**
 * Recursively walk a predicate AST and register/unregister index watchers and
 * wildcard watchers for the given watcher ID pair.
 *
 * Rust: pub fn recurse_predicate_watchers(
 *          &mut self,
 *          collection_id: &CollectionId,
 *          predicate: &Predicate,
 *          watcher_id: (ReactorSubscriptionId, QueryId),
 *          op: WatcherOp,
 *       )
 *
 * @param collectionId  The collection this predicate targets.
 * @param predicate     The AnkQL predicate AST node.
 * @param watcherId     The (subscriptionId, queryId) pair to register.
 * @param op            'Add' or 'Remove'.
 */
recursePredicateWatchers(
  collectionId: CollectionId,
  predicate: Predicate,
  watcherId: WatcherIdPair,
  op: WatcherOp,
): void
```

**Behavior by predicate variant:**

| Predicate variant | Action |
|---|---|
| `Comparison { left, operator, right }` | If one side is `Path` and the other is `Literal` (in either order): build a `PropertyPath` from the path, compute the composite key, get-or-create the `ComparisonIndex<string>` for that key, then `add`/`remove` using the literal, operator, and the `watcherIdPairKey`. Also register/deregister the pair in `watcherIdLookup`. If both sides are non-Path/Literal, silently skip (mirrors Rust behavior). |
| `And(left, right)` | Recurse into both children. |
| `Or(left, right)` | Recurse into both children. |
| `Not(predicate)` | Recurse into the inner predicate. |
| `True` | Get-or-create the wildcard set for `collectionId`. On `Add`, insert the pair; on `Remove`, delete it. |
| `False` | `throw new Error('Predicate::False not implemented')` (matches Rust `unimplemented!`). |
| `IsNull` | `throw new Error('Predicate::IsNull not implemented')` (matches Rust `unimplemented!`). |
| `Placeholder` | `throw new Error('Placeholder should be transformed before reactor processing')`. |

### 6.2 `accumulateInterestedWatchers()`

The core fan-out method. Given an entity and its offset in the shared changes array, finds all subscriptions that *might* be interested and populates their `CandidateChanges`.

```ts
/**
 * For a single entity change, find all subscriptions that might be interested
 * and record this change in their CandidateChanges accumulator.
 *
 * Rust: pub fn accumulate_interested_watchers<E: AbstractEntity, C>(
 *          &self,
 *          entity: &E,
 *          offset: usize,
 *          changes_arc: &Arc<Vec<C>>,
 *          candidates_by_sub: &mut HashMap<ReactorSubscriptionId, CandidateChanges<C>>,
 *       )
 *
 * Divergence from Rust:
 * - `Arc<Vec<C>>` -> `readonly C[]` (JS passes arrays by reference, no need for Arc).
 * - `HashMap<ReactorSubscriptionId, CandidateChanges<C>>` -> `Map<string, CandidateChanges<C>>`
 *   keyed by `subscriptionId.toKey()`, with a parallel Map for the actual subscriptionId objects.
 * - `E: AbstractEntity` -> `Entity` (concrete type; the TS port uses Entity directly).
 *
 * @param entity           The entity that changed.
 * @param offset           Index of this change in the shared `changes` array.
 * @param changes          The shared changes array (same reference used for all CandidateChanges).
 * @param candidatesBySub  Accumulator map: subscriptionId key -> CandidateChanges.
 *                         Created entries reference the shared `changes` array.
 */
accumulateInterestedWatchers<C>(
  entity: Entity,
  offset: number,
  changes: readonly C[],
  candidatesBySub: Map<string, { subscriptionId: ReactorSubscriptionId; candidates: CandidateChanges<C> }>,
): void
```

**Algorithm (three phases, executed sequentially for each entity):**

#### Phase 1: Index watchers

```
for each (compositeKey, comparisonIndex) in this.indexWatchers:
  parse the collectionId from compositeKey
  if entity.collectionId !== that collectionId:
    continue

  parse the propertyPath from compositeKey (or store it alongside the index)
  let value = propertyPath.extractValue(entity)
  if value is null:
    continue

  let matchingKeys: string[] = comparisonIndex.findMatching(value)
  for each key in matchingKeys:
    let pair = this.watcherIdLookup.get(key)
    let entry = candidatesBySub.get(pair.subscriptionId.toKey())
                 ?? create new { subscriptionId: pair.subscriptionId, candidates: new CandidateChanges(changes) }
    entry.candidates.addQuery(pair.queryId, offset)
```

**Implementation note:** To avoid re-parsing compositeKey on every call, store the `CollectionId` and `PropertyPath` alongside the `ComparisonIndex` in the Map value:

```ts
private indexWatchers: Map<string, {
  collectionId: CollectionId;
  propertyPath: PropertyPath;
  index: ComparisonIndex<string>;
}> = new Map();
```

#### Phase 2: Wildcard watchers

```
let collectionKey = entity.collectionId.toString()
let wildcards = this.wildcardWatchers.get(collectionKey)
if wildcards exists:
  for each (key, pair) in wildcards:
    let entry = candidatesBySub.get(pair.subscriptionId.toKey())
                 ?? create new { subscriptionId, candidates: new CandidateChanges(changes) }
    entry.candidates.addQuery(pair.queryId, offset)
```

#### Phase 3: Entity watchers

```
let entityKey = entity.entityId.toBase64()
let watchers = this.entityWatchers.get(entityKey)
if watchers exists:
  for each (key, watcherId) in watchers:
    switch watcherId.type:
      case 'Predicate':
        let entry = candidatesBySub.get(watcherId.subscriptionId.toKey())
                     ?? create new { ... }
        entry.candidates.addQuery(watcherId.queryId, offset)
      case 'Subscription':
        let entry = candidatesBySub.get(watcherId.subscriptionId.toKey())
                     ?? create new { ... }
        entry.candidates.addEntity(offset)
```

### 6.3 `applyWatcherChange()`

Applies a deferred `WatcherChange` to the entity watcher registry.

```ts
/**
 * Apply a single WatcherChange (add or remove an entity-level predicate watcher).
 *
 * Rust: pub fn apply_watcher_change(&mut self, change: WatcherChange)
 */
applyWatcherChange(change: WatcherChange): void
```

**Behavior:**

- **`Add`:** Get-or-create the entity watcher set for `change.entityId`. Insert `EntityWatcherId::Predicate(change.subscriptionId, change.queryId)`.
- **`Remove`:** Look up the entity watcher set for `change.entityId`. Remove the matching `Predicate` entry. If the set is now empty, delete the entity entry entirely (prevents memory leak).

### 6.4 `addEntitySubscription()`

```ts
/**
 * Register an entity-level subscription watcher (not tied to any query predicate).
 *
 * Rust: pub fn add_entity_subscription(&mut self, subscription_id: ReactorSubscriptionId, entity_id: EntityId)
 */
addEntitySubscription(subscriptionId: ReactorSubscriptionId, entityId: EntityId): void
```

Inserts `EntityWatcherId::Subscription(subscriptionId)` into the entity watcher set for `entityId`.

### 6.5 `removeEntitySubscription()`

```ts
/**
 * Remove an entity-level subscription watcher.
 *
 * Rust: pub fn remove_entity_subscription(&mut self, subscription_id: ReactorSubscriptionId, entity_id: EntityId)
 */
removeEntitySubscription(subscriptionId: ReactorSubscriptionId, entityId: EntityId): void
```

Removes the `Subscription(subscriptionId)` entry. Cleans up the entity entry if now empty.

### 6.6 `removeEntitySubscriptions()` (batch)

```ts
/**
 * Remove entity subscription watchers for multiple entities.
 *
 * Rust: pub fn remove_entity_subscriptions(&mut self, subscription_id, entity_ids: impl IntoIterator<Item = EntityId>)
 */
removeEntitySubscriptions(subscriptionId: ReactorSubscriptionId, entityIds: Iterable<EntityId>): void
```

Loops over `entityIds` and calls `removeEntitySubscription` for each.

### 6.7 `addPredicateEntityWatchers()` (batch)

```ts
/**
 * Add predicate-based entity watchers for multiple entities at once.
 *
 * Rust: pub fn add_predicate_entity_watchers(
 *          &mut self, subscription_id, query_id, entity_ids: impl IntoIterator<Item = EntityId>
 *       )
 */
addPredicateEntityWatchers(
  subscriptionId: ReactorSubscriptionId,
  queryId: QueryId,
  entityIds: Iterable<EntityId>,
): void
```

For each entity ID, inserts `EntityWatcherId::Predicate(subscriptionId, queryId)` into the entity watcher set.

### 6.8 `cleanupRemovedPredicateWatchers()`

```ts
/**
 * Remove predicate entity watchers for entities that no longer match a query.
 *
 * Rust: pub fn cleanup_removed_predicate_watchers(
 *          &mut self, subscription_id, query_id, removed_entities: &[EntityId]
 *       )
 */
cleanupRemovedPredicateWatchers(
  subscriptionId: ReactorSubscriptionId,
  queryId: QueryId,
  removedEntities: readonly EntityId[],
): void
```

For each entity in `removedEntities`, removes the matching `Predicate(subscriptionId, queryId)` entry from that entity's watcher set.

**Note:** Unlike `removeEntitySubscription`, this does NOT clean up empty entity entries. The Rust code does not call `self.entity_watchers.remove(&entity_id)` when the set becomes empty in this method. However, for consistency and to avoid memory leaks, the TS port SHOULD clean up empty entries.

### 6.9 `clearEntityWatchers()`

```ts
/**
 * Clear ALL entity watchers. Used during system reset.
 *
 * Rust: pub fn clear_entity_watchers(&mut self)
 */
clearEntityWatchers(): void
```

Calls `this.entityWatchers.clear()`.

### 6.10 `debugData()`

```ts
/**
 * Return references to internal data for debugging/testing.
 *
 * Rust: pub fn debug_data(&self) -> (&index_watchers, &wildcard_watchers, &entity_watchers)
 *
 * Divergence: Returns the internal maps directly (JS has no borrow checker concern).
 */
debugData(): {
  indexWatchers: Map<string, { collectionId: CollectionId; propertyPath: PropertyPath; index: ComparisonIndex<string> }>;
  wildcardWatchers: Map<string, Map<string, WatcherIdPair>>;
  entityWatchers: Map<string, Map<string, EntityWatcherId>>;
}
```

---

## 7. How `WatcherSet` Uses `ComparisonIndex` and `PropertyPath`

### ComparisonIndex

- **Type parameter:** `ComparisonIndex<string>` where the string is a serialized `WatcherIdPair` key.
- **`add(literal, operator, watcherKey)`**: Called from `recursePredicateWatchers()` when a `Comparison` node has a `Path` and `Literal`.
- **`remove(literal, operator, watcherKey)`**: Called from `recursePredicateWatchers()` for `WatcherOp.Remove`.
- **`findMatching(probeValue)`**: Called from `accumulateInterestedWatchers()` Phase 1 to find all watcher keys whose comparison conditions match the entity's property value.

### PropertyPath

- **`PropertyPath.fromPath(pathExpr)`**: Called in `recursePredicateWatchers()` to convert a `PathExpr` from the AST into a `PropertyPath`.
- **`propertyPath.extractValue(entity)`**: Called in `accumulateInterestedWatchers()` Phase 1 to get the entity's value at the indexed property path.
- **`propertyPath.toString()`**: Used as part of the composite key for the `indexWatchers` map.

---

## 8. Concurrency Patterns: Rust -> JS Simplification

| Rust pattern | JS equivalent | Notes |
|---|---|---|
| `Arc<Mutex<WatcherSet>>` | Plain `WatcherSet` field on `Reactor` | JS is single-threaded; no locking needed. |
| `Arc<Vec<C>>` (shared changes array) | `readonly C[]` | JS arrays are reference types; shared via reference. |
| `HashMap<K, V>` with composite tuple keys `(CollectionId, PropertyPath)` | `Map<string, V>` with serialized string keys | JS Maps use reference equality; must serialize composite keys. |
| `HashSet<T>` where T has `Hash + Eq` | `Map<string, T>` keyed by serialized value | Same reference-equality issue. |
| `Mutex` guards in `notify_change` serializing access | Unnecessary in JS | JS event loop ensures single-threaded access. If needed for async ordering, use a simple queue/promise chain. |
| `tokio::sync::Mutex` for `notify_lock` | Promise-based sequencing or unnecessary | Only needed if `notifyChange` is async and must serialize. In browser/Deno JS, microtask ordering may suffice. |
| `Arc::clone` for sharing | Direct reference sharing | JS GC handles lifetime. |

---

## 9. Complete Type Summary

### Types Exported from `watcher-set.ts`

| Type | Kind | Description |
|---|---|---|
| `WatcherOp` | Type alias (`'Add' \| 'Remove'`) | Add or remove operation. |
| `EntityWatcherId` | Discriminated union | Predicate or Subscription variant. |
| `WatcherChange` | Discriminated union | Deferred Add/Remove for entity watchers. |
| `WatcherIdPair` | Interface | `{ subscriptionId, queryId }` tuple. |
| `WatcherSet` | Class | The main routing table. |

### Types Imported

| Type | From |
|---|---|
| `CollectionId`, `EntityId`, `QueryId` | `@ankurah/proto` |
| `Predicate`, `ComparisonOperator`, `Literal`, `PathExpr`, `Expr` | `@ankurah/ankql` |
| `ComparisonIndex` | `./comparison-index.ts` |
| `PropertyPath` | `./property-path.ts` |
| `CandidateChanges` | `./candidate-changes.ts` |
| `Entity` | `../entity.ts` |
| `Value` | `../value/index.ts` |
| `ReactorSubscriptionId` | (to be defined, see section 3) |

### Functions Exported from `watcher-set.ts`

| Function | Signature | Purpose |
|---|---|---|
| `entityWatcherSubscriptionId` | `(ew: EntityWatcherId) => ReactorSubscriptionId` | Extract sub ID from either variant. |
| `entityWatcherIdKey` | `(ew: EntityWatcherId) => string` | Stable string key for Set/Map usage. |
| `watcherIdPairKey` | `(pair: WatcherIdPair) => string` | Stable string key for the (subId, queryId) tuple. |
| `watcherChangeAdd` | `(entityId, subscriptionId, queryId) => WatcherChange` | Factory for Add variant. |
| `watcherChangeRemove` | `(entityId, subscriptionId, queryId) => WatcherChange` | Factory for Remove variant. |

---

## 10. `accumulateInterestedWatchers` Detailed Pseudocode

This is the most critical method. Here is the complete step-by-step:

```
function accumulateInterestedWatchers<C>(
  entity: Entity,
  offset: number,
  changes: readonly C[],
  candidatesBySub: Map<string, { subscriptionId: ReactorSubscriptionId; candidates: CandidateChanges<C> }>,
): void {
  const entityId = entity.entityId;
  const entityCollectionStr = entity.collectionId.toString();

  // ── Phase 1: Index watchers ──
  for (const [_key, { collectionId, propertyPath, index }] of this.indexWatchers) {
    if (collectionId.toString() !== entityCollectionStr) continue;

    const value: Value | null = propertyPath.extractValue(entity);
    if (value === null) continue;

    const matchingKeys: string[] = index.findMatching(value);
    for (const watcherKey of matchingKeys) {
      const pair = this.watcherIdLookup.get(watcherKey)!;
      const subKey = pair.subscriptionId.toKey();
      let entry = candidatesBySub.get(subKey);
      if (!entry) {
        entry = { subscriptionId: pair.subscriptionId, candidates: new CandidateChanges(changes) };
        candidatesBySub.set(subKey, entry);
      }
      entry.candidates.addQuery(pair.queryId, offset);
    }
  }

  // ── Phase 2: Wildcard watchers ──
  const wildcards = this.wildcardWatchers.get(entityCollectionStr);
  if (wildcards) {
    for (const [_key, pair] of wildcards) {
      const subKey = pair.subscriptionId.toKey();
      let entry = candidatesBySub.get(subKey);
      if (!entry) {
        entry = { subscriptionId: pair.subscriptionId, candidates: new CandidateChanges(changes) };
        candidatesBySub.set(subKey, entry);
      }
      entry.candidates.addQuery(pair.queryId, offset);
    }
  }

  // ── Phase 3: Entity watchers ──
  const entityKey = entityId.toBase64();
  const entityWatcherSet = this.entityWatchers.get(entityKey);
  if (entityWatcherSet) {
    for (const [_key, watcherId] of entityWatcherSet) {
      const subKey = watcherId.subscriptionId.toKey();
      let entry = candidatesBySub.get(subKey);
      if (!entry) {
        entry = {
          subscriptionId: watcherId.subscriptionId,
          candidates: new CandidateChanges(changes),
        };
        candidatesBySub.set(subKey, entry);
      }

      if (watcherId.type === 'Predicate') {
        entry.candidates.addQuery(watcherId.queryId, offset);
      } else {
        // Subscription -- entity-level, not tied to a query
        entry.candidates.addEntity(offset);
      }
    }
  }
}
```

---

## 11. `recursePredicateWatchers` Detailed Pseudocode

```
function recursePredicateWatchers(
  collectionId: CollectionId,
  predicate: Predicate,
  watcherId: WatcherIdPair,
  op: WatcherOp,
): void {
  const pairKey = watcherIdPairKey(watcherId);

  switch (predicate.type) {
    case 'Comparison': {
      // Extract path and literal from left/right (in either order)
      let path: PathExpr | null = null;
      let literal: Literal | null = null;
      let operator: ComparisonOperator = predicate.operator;

      const { left, right } = predicate;
      if (left.type === 'Path' && right.type === 'Literal') {
        path = left.value;
        literal = right.value;
      } else if (left.type === 'Literal' && right.type === 'Path') {
        path = right.value;
        literal = left.value;
      }

      if (path && literal) {
        const propertyPath = PropertyPath.fromPath(path);
        const compositeKey = indexWatcherKey(collectionId, propertyPath);
        let entry = this.indexWatchers.get(compositeKey);
        if (!entry) {
          entry = { collectionId, propertyPath, index: new ComparisonIndex<string>() };
          this.indexWatchers.set(compositeKey, entry);
        }

        if (op === 'Add') {
          entry.index.add(literal, operator, pairKey);
          this.watcherIdLookup.set(pairKey, watcherId);
        } else {
          entry.index.remove(literal, operator, pairKey);
          // Note: Do NOT remove from watcherIdLookup here because the same
          // pair may be registered in multiple indexes. Cleanup happens when
          // subscription is fully removed.
        }
      }
      // else: unsupported comparison shape, silently skip
      break;
    }

    case 'And':
    case 'Or': {
      this.recursePredicateWatchers(collectionId, predicate.left, watcherId, op);
      this.recursePredicateWatchers(collectionId, predicate.right, watcherId, op);
      break;
    }

    case 'Not': {
      this.recursePredicateWatchers(collectionId, predicate.predicate, watcherId, op);
      break;
    }

    case 'True': {
      const collectionKey = collectionId.toString();
      let set = this.wildcardWatchers.get(collectionKey);
      if (!set) {
        set = new Map();
        this.wildcardWatchers.set(collectionKey, set);
      }

      if (op === 'Add') {
        set.set(pairKey, watcherId);
      } else {
        set.delete(pairKey);
      }
      break;
    }

    case 'IsNull':
      throw new Error('Predicate::IsNull not implemented in WatcherSet');

    case 'False':
      throw new Error('Predicate::False not implemented in WatcherSet');

    case 'Placeholder':
      throw new Error('Placeholder should be transformed before reactor processing');
  }
}
```

---

## 12. Design Notes

### Why three registries?

The three-tier design optimizes the hot path:

1. **Index watchers** handle the common case of `field = value` or `field > value` comparisons without scanning all subscriptions.
2. **Wildcard watchers** handle `SELECT * FROM collection` (no WHERE clause / `Predicate::True`) by collection ID.
3. **Entity watchers** provide O(1) lookup for entities that are *already known* to match a subscription, avoiding repeated predicate evaluation.

### Ordering of accumulation

The three phases are independent and additive -- the same subscription can appear in candidates from index watchers AND entity watchers for the same entity change. The `CandidateChanges.addQuery()` call simply records the offset; deduplication happens later in `evaluate_changes`.

### Memory management

The `watcherIdLookup` map grows monotonically as watchers are added. When a subscription is fully removed (unsubscribe), the caller should clean up entries. Consider adding a `removeWatcherIdPair(pairKey)` method or performing cleanup in `recursePredicateWatchers` when `op === 'Remove'` and we can confirm the pair is no longer referenced by any index.

### Entity ID keying

`EntityId.toBase64()` is used as the stable string key for entity watcher maps. This matches the existing pattern in `CandidateChanges` which uses `QueryId.toUlidString()`.
