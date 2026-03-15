# Reactor Main - Porting Spec

**Source:** `ankurah/core/src/reactor.rs` (629 lines), `subscription_state.rs`, `subscription.rs`, `watcherset.rs`
**Target:** `ankurah-ts/packages/core/src/reactor/reactor.ts` (new file)

---

## 1. Types Overview

### 1.1 AbstractEntity trait (Rust) -- NOT needed in TS

```rust
pub trait AbstractEntity: Clone + std::fmt::Debug {
    fn collection(&self) -> proto::CollectionId;
    fn id(&self) -> &proto::EntityId;
    fn value(&self, field: &str) -> Option<Value>;
}
```

**TS decision:** The TS port uses concrete `Entity` everywhere. Entity already has:
- `entity.collectionId` / `entity.collection()` -> `CollectionId`
- `entity.entityId` / `entity.id()` -> `EntityId`
- `entity.getPropertyValue(field)` -> `Value | null`

The `Filterable` interface in `selection/filter.ts` serves the same purpose:
```ts
export interface Filterable {
    collection(): string;
    value(name: string): Value | null;
}
```

Entity does NOT currently implement `Filterable`. The evaluatePredicate function takes `Filterable`, so the Reactor will need to adapt Entity to Filterable. Options:
1. Make Entity implement Filterable (preferred -- add `value()` method that delegates to `getPropertyValue()` and `collection()` that returns `collectionId.toString()`)
2. Create an adapter wrapper

**Recommendation:** Add a helper function `entityAsFilterable(entity: Entity): Filterable` or add the methods directly to Entity. The `evaluatePredicate` function already accepts `Filterable`.

### 1.2 ChangeNotification trait (Rust)

```rust
pub trait ChangeNotification: std::fmt::Debug + std::fmt::Display {
    type Entity: AbstractEntity;
    type Event: Clone + std::fmt::Debug;
    fn into_parts(self) -> (Self::Entity, Vec<Self::Event>);
    fn entity(&self) -> &Self::Entity;
    fn events(&self) -> &[Self::Event];
}
```

**TS equivalent:** The existing `EntityChange` class in `changes.ts` already mirrors this:
```ts
class EntityChange {
    readonly entity: Entity;
    readonly events: ReadonlyArray<Attested<Event>>;
    intoParts(): [Entity, Attested<Event>[]];
}
```

For `notify_change`, the TS port should use `EntityChange` directly. If a TS interface is desired:

```ts
interface ChangeNotification {
    readonly entity: Entity;
    readonly events: ReadonlyArray<Attested<Event>>;
    intoParts(): [Entity, Attested<Event>[]];
}
```

`EntityChange` already satisfies this. No new type needed.

### 1.3 PreNotifyHook trait (Rust)

```rust
pub trait PreNotifyHook {
    fn pre_notify(&self, version: u32);
}
impl PreNotifyHook for () {
    fn pre_notify(&self, _version: u32) {}
}
```

**TS equivalent:** A simple callback or null:
```ts
type PreNotifyHook = ((version: number) => void) | null;
```

The no-op case (Rust `()`) maps to `null` or an omitted parameter.

### 1.4 ReactorSubscriptionId (already ported)

Already exists in `watcher_set.ts`. Uses ULID string as identity. No changes needed.

### 1.5 Reactor struct

```rust
pub struct Reactor<E, Ev>(Arc<ReactorInner<E, Ev>>);

struct ReactorInner<E, Ev> {
    subscriptions: std::sync::Mutex<HashMap<ReactorSubscriptionId, Subscription<E, Ev>>>,
    watcher_set: Arc<std::sync::Mutex<WatcherSet>>,
    notify_lock: tokio::sync::Mutex<()>,
}
```

**TS equivalent:** No generics (concrete Entity + Attested<Event>). No Arc/Mutex.

```ts
class Reactor {
    private subscriptions: Map<string, Subscription>;   // key = ReactorSubscriptionId.toKey()
    private watcherSet: WatcherSet;                       // plain field, no Mutex
    private notifyLock: PromiseMutex;                     // serialization lock (see Section 7)
}
```

### 1.6 Subscription (internal, from subscription_state.rs)

```rust
struct Inner<E: AbstractEntity + Filterable, Ev> {
    id: ReactorSubscriptionId,
    state: Mutex<State<E, Ev>>,
    watcher_set: Arc<Mutex<WatcherSet>>,
}

struct State<E, Ev> {
    queries: HashMap<QueryId, QueryState<E>>,
    entity_subscriptions: HashSet<EntityId>,
    entities: HashMap<EntityId, E>,
    broadcast: Broadcast<ReactorUpdate<E, Ev>>,
}
```

**TS equivalent:**
```ts
class Subscription {
    readonly id: ReactorSubscriptionId;
    private queries: Map<string, QueryState>;           // key = QueryId.toUlidString()
    private entitySubscriptions: Set<string>;           // EntityId.toBase64() keys
    private entities: Map<string, Entity>;              // EntityId.toBase64() -> Entity
    readonly broadcast: Broadcast<ReactorUpdate>;
    private watcherSet: WatcherSet;                     // shared reference (same object as Reactor's)
}
```

### 1.7 QueryState (from subscription_state.rs)

```rust
pub struct QueryState<E: AbstractEntity + Filterable> {
    pub(crate) collection_id: proto::CollectionId,
    pub(crate) selection: Option<ankql::ast::Selection>,
    pub(crate) gap_fetcher: Arc<dyn GapFetcher<E>>,
    pub(crate) paused: bool,
    pub(crate) resultset: EntityResultSet<E>,
    pub(crate) version: u32,
}
```

**TS equivalent:**
```ts
interface QueryState {
    collectionId: CollectionId;
    selection: Selection | null;            // null until first updateQuery
    gapFetcher: GapFetcher;
    paused: boolean;
    resultset: EntityResultSet;
    version: number;
}
```

### 1.8 ReactorSubscription (public handle, from subscription.rs)

```rust
pub struct ReactorSubscription<E, Ev>(Arc<ReactorSubInner<E, Ev>>);

struct ReactorSubInner<E, Ev> {
    subscription_id: ReactorSubscriptionId,
    reactor: Reactor<E, Ev>,
    broadcast: Broadcast<ReactorUpdate<E, Ev>>,
}
```

Drop impl calls `reactor.unsubscribe(subscription_id)`.

**TS equivalent:**
```ts
class ReactorSubscription {
    readonly subscriptionId: ReactorSubscriptionId;
    private reactor: Reactor;
    readonly broadcast: Broadcast<ReactorUpdate>;

    // Methods
    id(): ReactorSubscriptionId;
    removePredicate(queryId: QueryId): void;
    addEntitySubscriptions(entityIds: Iterable<EntityId>): void;
    removeEntitySubscriptions(entityIds: Iterable<EntityId>): void;
    subscribe(listener: (update: ReactorUpdate) => void): ListenerGuard<ReactorUpdate>;

    // Manual cleanup (replaces Rust Drop)
    dispose(): void;   // calls reactor.unsubscribe(this.subscriptionId)
}
```

**Note:** Implements `Signal` and `Subscribe<ReactorUpdate>` from the signals library. In TS:
- `listen(listener: Listener): ListenerGuard` -- NotifyOnly path
- `subscribe(callback: (update: ReactorUpdate) => void): ListenerGuard<ReactorUpdate>` -- Payload path

### 1.9 UpdateItemAccumulator trait (from subscription_state.rs)

```rust
pub trait UpdateItemAccumulator<E, Ev> {
    fn push_initial(&mut self, entity: &E, query_id: QueryId);
    fn push_remove(&mut self, entity: &E, query_id: QueryId);
}
```

Implemented for `Vec<ReactorUpdateItem>` (collects items) and `()` (discards).

**TS equivalent:** Use a simple interface or union approach:
```ts
interface UpdateItemAccumulator {
    pushInitial(entity: Entity, queryId: QueryId): void;
    pushRemove(entity: Entity, queryId: QueryId): void;
}
```

With concrete implementations:
- `ArrayAccumulator` wrapping `ReactorUpdateItem[]`
- `NoopAccumulator` as the `()` equivalent

Alternatively, use `ReactorUpdateItem[] | null` and check before pushing.

### 1.10 GapFillData tuple (from subscription_state.rs)

```rust
type GapFillData<E> = (
    QueryId,
    Arc<dyn GapFetcher<E>>,
    CollectionId,
    Selection,
    EntityResultSet<E>,
    Option<E>,   // last entity
    usize,       // gap size
);
```

**TS equivalent:**
```ts
interface GapFillData {
    queryId: QueryId;
    gapFetcher: GapFetcher;
    collectionId: CollectionId;
    selection: Selection;
    resultset: EntityResultSet;
    lastEntity: Entity | null;
    gapSize: number;
}
```

---

## 2. All Methods with Full Signatures and Behavior

### 2.1 Reactor methods

#### `new()` / constructor
```ts
constructor()
```
Creates empty subscriptions map, new WatcherSet, and initializes the notify lock.

#### `subscribe(): ReactorSubscription`
1. Create a new `Broadcast<ReactorUpdate>`.
2. Create a new `Subscription` with that broadcast and a reference to the shared `watcherSet`.
3. Insert subscription into `this.subscriptions` map.
4. Return a new `ReactorSubscription` handle wrapping the subscription ID, reactor reference, and broadcast.

#### `unsubscribe(subId: ReactorSubscriptionId): void`
1. Remove subscription from map. Throw `SubscriptionError` if not found.
2. Take all queries from the subscription (`takeAllQueries()`).
3. For each query with a non-null selection: call `watcherSet.recursePredicateWatchers(collectionId, predicate, watcherId, 'Remove')`.
4. For each query: collect entity IDs from its resultset keys, call `watcherSet.removeEntitySubscriptions(subId, entityIds)`.

#### `removeQuery(subscriptionId, queryId): void`
1. Look up subscription. Throw if not found.
2. Call `subscription.removeQuery(queryId)`. Throw if query not found.
3. If the removed query had a selection: call `watcherSet.recursePredicateWatchers(collectionId, predicate, watcherId, 'Remove')`.

#### `addEntitySubscriptions(subscriptionId, entityIds: Iterable<EntityId>): void`
1. Look up subscription. Return silently if not found.
2. For each entity ID: call `subscription.addEntitySubscription(entityId)` and `watcherSet.addEntitySubscription(subscriptionId, entityId)`.

#### `removeEntitySubscriptions(subscriptionId, entityIds: Iterable<EntityId>): void`
1. Look up subscription. Return silently if not found.
2. For each entity ID:
   - Call `subscription.removeEntitySubscription(entityId)`.
   - Check if any queries still match this entity (`subscription.anyQueryMatches(entityId)`).
   - If none match: call `watcherSet.removeEntitySubscription(subscriptionId, entityId)`.

#### `addQueryAndNotify(...)` -- async
```ts
async addQueryAndNotify(
    subscriptionId: ReactorSubscriptionId,
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    node: Node,                    // or TNodeErased equivalent
    resultset: EntityResultSet,
    gapFetcher: GapFetcher,
    preNotifyHook: PreNotifyHook,
): Promise<void>
```

Behavior:
1. Look up subscription. Throw if not found.
2. Fetch initial entities: `await node.fetchEntitiesFromLocal(collectionId, selection)`.
3. Register empty query: `subscription.registerQuery(queryId, collectionId, resultset, gapFetcher)`.
4. Call `subscription.updateQuery(queryId, collectionId, selection, includedEntities, 1, accumulator)` to populate resultset and collect ReactorUpdateItems.
5. Fill gaps: `await subscription.fillGapsForQuery(queryId, accumulator)`.
6. Set loaded: `resultset.setLoaded(true)`.
7. Call preNotifyHook (if not null) with version=1.
8. Send notification: `subscription.sendUpdate(reactorUpdateItems)`. Always notify on initial.

#### `updateQueryAndNotify(...)` -- async
```ts
async updateQueryAndNotify(
    subscriptionId: ReactorSubscriptionId,
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    node: Node,
    version: number,
    preNotifyHook: PreNotifyHook,
): Promise<void>
```

Behavior:
1. Fetch entities: `await node.fetchEntitiesFromLocal(collectionId, selection)`.
2. Look up subscription. Throw if not found.
3. Call `subscription.updateQuery(queryId, collectionId, selection, includedEntities, version, accumulator)`.
4. Fill gaps: `await subscription.fillGapsForQuery(queryId, accumulator)`.
5. Call preNotifyHook with version.
6. Send update only if items is non-empty.

#### `notifyChange(changes: EntityChange[])` -- async
See Section 3 for the three-phase pipeline.

#### `systemReset(): void`
1. Clear entity watchers: `watcherSet.clearEntityWatchers()`.
2. For each subscription: call `subscription.systemReset()`.

### 2.2 Subscription methods (internal)

#### `constructor(broadcast, watcherSet)`
Create with new ID, empty queries/entities/entitySubscriptions maps.

#### `registerQuery(queryId, collectionId, resultset, gapFetcher): void`
Insert a new QueryState with `selection: null, paused: false, version: 0`. Error if query already exists.

#### `updateQuery(queryId, collectionId, selection, includedEntities, version, accumulator): Entity[]`
This is the most complex method. Steps:
1. Look up query state. Error if not found.
2. Check if first update (`selection === null`).
3. Save old selection, set new selection.
4. Update resultset ordering from selection's `orderBy` (calls `buildKeySpecFromSelection`).
5. Set limit if first update or if limit changed.
6. Create write guard: `resultset.write()`.
7. `markAllDirty()`.
8. Process included entities:
   - For each, check `evaluatePredicate(entity, selection.predicate)`.
   - If matches and not already in resultset: add to resultset, cache in `entities` map, add to `entitySubscriptions`, push Initial to accumulator, collect in `newlyAdded`.
9. Retain dirty: `retainDirty()` removes entities no longer matching. Push Remove to accumulator for each.
10. Unpause, set version.
11. `setLoaded(true)` inside write guard. Drop write guard (`done()`).
12. Update predicate watchers if first update or predicate changed.
13. Add entity watchers for newly added entities.
14. Remove entity watchers for removed entities.
15. Return `newlyAdded`.

#### `updatePredicateWatchers(queryId, collectionId, oldPredicate, newPredicate): void`
Calls `watcherSet.recursePredicateWatchers` to remove old and add new.

#### `addEntityWatchers(queryId, entityIds: Iterable<EntityId>): void`
Calls `watcherSet.addPredicateEntityWatchers(this.id, queryId, entityIds)`.

#### `evaluateChanges(candidates: CandidateChanges<EntityChange>): Promise<WatcherChange[]>`
See Section 3 (Phase 2). This is called per-subscription. Returns watcher changes.

#### `sendUpdate(items: ReactorUpdateItem[]): void`
Sends `ReactorUpdate { items }` via `broadcast.send()`.

#### `removeQuery(queryId): QueryState | null`
Removes from queries map, returns the state.

#### `takeAllQueries(): Map<string, QueryState>`
Drains all queries from the map.

#### `systemReset(): void`
1. For each query, for each entity in its resultset, create Remove items.
2. Clear resultsets, set loaded=false.
3. Clear entity subscriptions and entities cache.
4. Send notification if any updates.

#### `anyQueryMatches(entityId): boolean`
Returns true if any query's resultset contains the entity.

#### `addEntitySubscription(entityId): void` / `removeEntitySubscription(entityId): void`
Add/remove from the `entitySubscriptions` set.

#### `collectGapsToFillInternal(): GapFillData[]`
Iterates queries, calls `extractGapData` for each.

#### `extractGapData(queryId, queryState): GapFillData | null`
Returns null if `!resultset.isGapDirty()` or if `currentLen >= limit`. Otherwise returns the tuple of data needed for gap filling.

#### `fillGapsForQuery(queryId, accumulator): Promise<void>`
1. Extract gap data for the specific query.
2. Clear gap_dirty flag.
3. Call `processGapFillEntities(...)`.
4. Add entity watchers for gap-filled entities.
5. Push Initial items to accumulator for each gap-filled entity.

#### `fillGapsAndNotify(items, gapsToFill, broadcast): Promise<void>`
Background task that:
1. Clears gap_dirty flags immediately.
2. Processes all gap fills concurrently (`Promise.all`).
3. Registers entity watchers for gap-filled entities.
4. Sends consolidated update.

#### `processGapFillEntities(queryId, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize): Promise<Entity[]>`
Static helper. Calls `gapFetcher.fetchGap(...)`, adds results to resultset, returns added entities.

---

## 3. The Three-Phase notify_change Pipeline

```ts
async notifyChange(changes: EntityChange[]): Promise<void>
```

### Phase 1: Accumulate Interested Watchers

**Lock:** Acquire the notify serialization lock (PromiseMutex).

1. Iterate over each change at index `offset`.
2. For each change, call `watcherSet.accumulateInterestedWatchers(change.entity, offset, changes, candidatesBySub)`.
3. This builds a `Map<string, { subscriptionId, candidates: CandidateChanges<EntityChange> }>` mapping each subscription to its candidate changes.

The `accumulateInterestedWatchers` method checks three registries:
- **Index watchers:** For each `(collection, propertyPath)` entry matching the entity's collection, extract the value at that path, find matching watcher IDs via `ComparisonIndex.findMatching(value)`, and add the change offset.
- **Wildcard watchers:** For the entity's collection, add the change offset to all wildcard watchers.
- **Entity watchers:** If the entity ID has registered watchers (predicate or subscription type), add the change offset.

### Phase 2: Evaluate Changes Per Subscription (parallelizable in Rust, sequential in JS)

For each subscription with candidates:

1. Call `subscription.evaluateChanges(candidates)`.
2. Inside `evaluateChanges`:
   a. Iterate over query candidates.
   b. For each query candidate, for each change:
      - Call `evaluatePredicate(entity, selection.predicate)` to check if entity matches NOW.
      - Check `resultset.containsKey(entityId)` to see if it matched BEFORE.
      - Determine membership change:
        - `(!didMatch, matches)` => Add: add to resultset, push WatcherChange.add
        - `(didMatch, !matches)` => Remove: remove from resultset, push WatcherChange.remove
        - Otherwise: push watcher change reflecting current state (add if matches, remove if not)
      - If matches OR didMatch OR entitySubscribed: add to output items
   c. Process entity-level subscriptions (not covered by query processing).
   d. Collect gap fill data while state is accessible.
   e. **Spawn background task** for gap filling + notification (or send immediately if no gaps).
3. Return `WatcherChange[]`.

**JS concurrency note:** In Rust, `evaluateChanges` calls are parallelized with `join_all`. In JS, they can either be run sequentially (simplest) or via `Promise.all` since each operates on independent subscription state. Since JS is single-threaded, the actual predicate evaluation work is synchronous, so `Promise.all` only helps if gap filling is involved.

### Phase 3: Apply Watcher Changes

After all evaluations complete:

1. Collect all `WatcherChange` arrays from all subscriptions (flatten).
2. For each `WatcherChange`, call `watcherSet.applyWatcherChange(change)`.

This updates the entity_watchers registry for subsequent `notifyChange` calls.

**Release** the notify serialization lock.

### Pipeline Diagram

```
changes[] ──> [Phase 1: accumulateInterestedWatchers]
                     │
                     v
            candidatesBySub (Map<SubId, CandidateChanges>)
                     │
                     v
              [Phase 2: evaluateChanges per subscription]
                     │
              ┌──────┴──────┐
              v              v
         WatcherChange[]   ReactorUpdateItems
              │              │
              v              v
   [Phase 3: apply]    [notify/gap-fill]
```

---

## 4. add_query / update_query / subscribe / unsubscribe Methods

### subscribe() -> ReactorSubscription
Creates a new subscription container. See Section 2.1.

### unsubscribe(subId) -> void
Removes subscription and cleans up all watchers. See Section 2.1.

### addQueryAndNotify(...) -> Promise<void>
Registers a new query for an existing subscription, fetches initial entities from storage, populates the resultset, fills gaps, and sends an initialization notification. This is used by `LiveQuery` when first created. See Section 2.1.

### updateQueryAndNotify(...) -> Promise<void>
Updates an existing query (e.g., when filter changes on a LiveQuery). Re-fetches from storage, diffs against the current resultset, fills gaps, and sends update notification. See Section 2.1.

### removeQuery(subscriptionId, queryId) -> void
Removes a single query from a subscription and cleans up its predicate/entity watchers. See Section 2.1.

### addEntitySubscriptions(subscriptionId, entityIds) -> void
Adds explicit entity-level subscriptions (not tied to any query predicate). These entities will always be included in notifications when they change, regardless of whether any query predicate matches.

### removeEntitySubscriptions(subscriptionId, entityIds) -> void
Removes entity-level subscriptions, but only if no query predicates still match the entity.

---

## 5. buildKeySpecFromSelection Helper

```rust
pub(crate) fn build_key_spec_from_selection<E: AbstractEntity>(
    order_by: &[OrderByItem],
    resultset: &EntityResultSet<E>,
) -> anyhow::Result<KeySpec>
```

**TS signature:**
```ts
function buildKeySpecFromSelection(
    orderBy: OrderByItem[],
    resultset: EntityResultSet,
): KeySpec
```

**Behavior:**
1. For each ORDER BY item:
   - Extract column name from `item.path.property()`.
   - Infer value type from first non-null value in resultset entities (`resultset.read().iterEntities()`).
   - Default to `ValueType.String` if no value found.
   - Map `Asc`/`Desc` to `IndexDirection`.
   - Create `IndexKeyPart { column, subPath: null, direction, valueType, nulls: 'Last', collation: null }`.
2. Return `KeySpec { keyparts }`.

**Dependencies:** `IndexKeyPart`, `KeySpec` from `indexing/index.ts`, `ValueType` from `value/index.ts`, `OrderByItem` from `@ankurah/ankql`.

---

## 6. How Reactor Integrates with Node

### Rust Architecture
In Rust, `NodeInner` holds `pub(crate) reactor: Reactor` as a direct field. The `Reactor` is created in `Node::new()` / `Node::new_durable()` and shared via `Arc<ReactorInner>`.

`TNodeErased` trait provides:
```rust
fn reactor(&self) -> &Reactor<E>;
fn fetch_entities_from_local(...) -> Result<Vec<E>, RetrievalError>;
fn unsubscribe_remote_predicate(...);
fn update_remote_query(...);
fn has_subscription_relay(&self) -> bool;
```

### Current TS Node
The TS `Node` class (`node.ts`) does NOT yet have a reactor field. The commit pipeline in `NodeAndContext.commitLocalTrx()` has:
```ts
// Phase 7: Reactor notification -- deferred until reactor is ported
```

### Integration Plan

1. **Add `reactor` field to `Node`:**
```ts
class Node {
    readonly reactor: Reactor;  // NEW
    // ... existing fields ...
    constructor(options) {
        this.reactor = new Reactor();
        // ...
    }
}
```

2. **Wire up commitLocalTrx Phase 7:**
After events are applied to canonical entities, create `EntityChange` objects and call:
```ts
await this.node.reactor.notifyChange(entityChanges);
```

3. **TNodeErased equivalent:**
The Rust `TNodeErased` trait is used by `add_query_and_notify` and `update_query_and_notify` to call `fetch_entities_from_local`. In TS, the `Node` class already has `fetchEntitiesFromLocal()`. The reactor methods can accept `Node` directly (no trait needed in TS).

However, if a `NodeLike` interface is desired for testability:
```ts
interface NodeLike {
    fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Entity[]>;
}
```

This is similar to the existing `NodeLike` in `fetch_gap.ts`.

4. **LiveQuery integration (future):**
LiveQuery will call `reactor.subscribe()` to get a `ReactorSubscription`, then `reactor.addQueryAndNotify()` to register its query. When the LiveQuery is disposed, it calls `reactorSubscription.dispose()`.

---

## 7. Concurrency Simplifications for JS

### Mutex -> plain fields

| Rust | TS |
|------|-----|
| `Mutex<HashMap<..., Subscription>>` | `Map<string, Subscription>` |
| `Arc<Mutex<WatcherSet>>` | `WatcherSet` (shared by reference) |
| `Mutex<State<E, Ev>>` (inside Subscription) | Plain fields on Subscription |

JS is single-threaded, so no synchronization primitives needed for data access.

### Arc -> plain reference / shared object

| Rust | TS |
|------|-----|
| `Arc<ReactorInner>` | Direct fields on `Reactor` class |
| `Arc<Mutex<WatcherSet>>` shared between Reactor and Subscriptions | Both hold reference to same `WatcherSet` object |
| `Arc<dyn GapFetcher>` | `GapFetcher` (interface reference) |
| `Arc<Vec<C>>` for changes sharing | `readonly C[]` (JS arrays are reference types) |

### tokio::sync::Mutex (notify_lock) -> PromiseMutex

The `notify_lock` is a **tokio async Mutex** (not std sync Mutex) used to serialize `notifyChange` calls. This matters because `notifyChange` is async and we need to prevent interleaving.

**TS implementation** -- a simple promise-based mutex:
```ts
class PromiseMutex {
    private queue: Promise<void> = Promise.resolve();

    async acquire(): Promise<() => void> {
        let release: () => void;
        const next = new Promise<void>(resolve => { release = resolve; });
        const prev = this.queue;
        this.queue = next;
        await prev;
        return release!;
    }
}
```

Usage in `notifyChange`:
```ts
const release = await this.notifyLock.acquire();
try {
    // ... three-phase pipeline ...
} finally {
    release();
}
```

### tokio::spawn -> fire-and-forget Promise / queueMicrotask

In `evaluateChanges`, Rust spawns a background task for gap filling + notification:
```rust
crate::task::spawn(self.clone().fill_gaps_and_notify(update_items, gaps_to_fill, broadcast));
```

**TS equivalent:** Fire-and-forget async call (no await). Since gap filling involves async I/O (fetching from storage), this should be:
```ts
// Fire and forget -- errors should be caught internally
this.fillGapsAndNotify(updateItems, gapsToFill, broadcast).catch(err => {
    console.error('Gap fill error:', err);
});
```

Or if gap filling should be awaited (safer, prevents race conditions):
```ts
await this.fillGapsAndNotify(updateItems, gapsToFill, broadcast);
```

**Recommendation:** Initially use `await` for simplicity. The notify_lock already serializes calls, so fire-and-forget gap fills could interleave with the next notifyChange. Using `await` is safer.

### join_all -> Promise.all

```rust
let all_watcher_changes: Vec<WatcherChange> = join_all(evaluations).await.into_iter().flatten().collect();
```

TS:
```ts
const results = await Promise.all(evaluations);
const allWatcherChanges = results.flat();
```

However, since JS is single-threaded and `evaluateChanges` is mostly synchronous (predicate evaluation), the benefit of `Promise.all` is minimal. Can simplify to a sequential loop.

### IndexMap -> Map (preserving insertion order)

Rust uses `IndexMap<EntityId, ReactorUpdateItem>` in `evaluateChanges` to preserve insertion order. JS `Map` already preserves insertion order, so `Map<string, ReactorUpdateItem>` works directly.

---

## 8. Import Dependencies from Existing TS Files

### From `@ankurah/proto`
```ts
import type { CollectionId, EntityId, QueryId, Attested, Event } from '@ankurah/proto';
```

### From `@ankurah/ankql`
```ts
import type { Selection, Predicate, OrderByItem } from '@ankurah/ankql';
```

### From `@ankurah/signals`
```ts
import { Broadcast, BroadcastRef, ListenerGuard, type BroadcastId, type Listener, type Signal } from '@ankurah/signals';
```

### From local packages (relative)
```ts
// Entity and related
import { Entity } from '../entity.ts';
import type { EntityChange } from '../changes.ts';

// Reactor submodules (already ported)
import { WatcherSet, ReactorSubscriptionId, type WatcherIdPair, type WatcherChange, watcherChangeAdd, watcherChangeRemove } from './watcher_set.ts';
import { CandidateChanges } from './candidate-changes.ts';
import type { ReactorUpdate, ReactorUpdateItem, MembershipChange } from './update.ts';
import type { GapFetcher } from './fetch_gap.ts';

// ResultSet
import { EntityResultSet } from '../resultset.ts';

// Selection/Filter
import { evaluatePredicate } from '../selection/filter.ts';
import type { Filterable } from '../selection/filter.ts';

// Indexing (for buildKeySpecFromSelection)
import type { KeySpec, IndexKeyPart } from '../indexing/index.ts';
import { IndexDirection, NullsOrder } from '../indexing/index.ts';

// Value types (for buildKeySpecFromSelection)
import { ValueType, valueType } from '../value/index.ts';

// Node (for addQueryAndNotify/updateQueryAndNotify)
import type { Node } from '../node.ts';
```

---

## 9. Forward References and Circular Dependency Concerns

### Reactor <-> Node circular dependency

**Problem:** `Reactor` needs `Node` for `addQueryAndNotify` / `updateQueryAndNotify` (to call `fetchEntitiesFromLocal`). `Node` needs `Reactor` as a field.

**Solution options:**
1. **Interface extraction:** Define a `NodeLike` interface with just `fetchEntitiesFromLocal()` in a shared location. Reactor depends on the interface, Node implements it. The existing `NodeLike` in `fetch_gap.ts` already does this.
2. **Parameter injection:** `addQueryAndNotify` receives a `fetchEntitiesFromLocal` callback instead of a Node reference.
3. **Lazy import:** Use dynamic import for one direction.
4. **Accept the cycle:** Many bundlers handle TS circular imports fine if exports are classes/interfaces (not values used at module evaluation time). Since `Node` only needs `Reactor` at construction time and `Reactor` only needs `Node` as a method parameter, this cycle is safe.

**Recommendation:** Option 1 -- use a `NodeLike` interface. Define it in a separate file (or reuse from `fetch_gap.ts`) to break the cycle.

### Reactor <-> Subscription circular dependency

**Not a problem:** `Subscription` is an internal class within the reactor module. It can live in the same file or in `subscription.ts` within the reactor directory. It references `WatcherSet` (already exists) and `Broadcast` (from signals). No circular dependency.

### ReactorSubscription <-> Reactor circular dependency

**Potential issue:** `ReactorSubscription` holds a reference to `Reactor` (for `dispose()` -> `unsubscribe()`), and `Reactor.subscribe()` returns `ReactorSubscription`.

**Solution:** Both can live in the same file. Or `ReactorSubscription` can be in a separate file that imports `Reactor` -- since `Reactor` only imports `ReactorSubscription` as a return type, the cycle is safe in TS (class declarations are hoisted).

### Entity <-> Filterable gap

**Issue:** `evaluatePredicate` takes `Filterable`, but `Entity` does not implement `Filterable`. The `Entity.getPropertyValue()` returns `Value | null` which matches `Filterable.value()`, but `Entity` has `collectionId: CollectionId` while `Filterable.collection()` returns `string`.

**Solution:** Either:
1. Add adapter: `function entityAsFilterable(e: Entity): Filterable`
2. Add methods to Entity: `collection(): string` (already exists as `collection(): CollectionId`; `Filterable` wants `string`)
3. Change `evaluatePredicate` to accept Entity directly (divergence from current code)

**Recommendation:** Create a simple adapter function. Entity already has `.collection()` returning `CollectionId` and `.getPropertyValue()`. The adapter wraps these:
```ts
function entityAsFilterable(entity: Entity): Filterable {
    return {
        collection: () => entity.collectionId.toString(),
        value: (name: string) => entity.getPropertyValue(name),
    };
}
```

### Existing exports from index.ts

The main `index.ts` already exports reactor submodules (comparison-index, property-path, candidate-changes, watcher_set, fetch_gap, update). The new `Reactor`, `ReactorSubscription`, and `Subscription` classes need to be added to exports.

---

## 10. Summary: Files to Create/Modify

### New file: `packages/core/src/reactor/reactor.ts`
Contains:
- `Reactor` class
- `Subscription` class (internal)
- `ReactorSubscription` class (public handle)
- `QueryState` interface
- `GapFillData` interface
- `UpdateItemAccumulator` interface + implementations
- `buildKeySpecFromSelection()` function
- `PromiseMutex` helper class
- `PreNotifyHook` type
- `entityAsFilterable()` adapter

### Modify: `packages/core/src/node.ts`
- Add `reactor: Reactor` field to `Node`
- Wire up Phase 7 in `commitLocalTrx` to call `reactor.notifyChange()`

### Modify: `packages/core/src/index.ts`
- Add exports for `Reactor`, `ReactorSubscription`

### No changes needed to:
- `watcher_set.ts` (already complete)
- `candidate-changes.ts` (already complete)
- `update.ts` (already complete)
- `fetch_gap.ts` (already complete)
- `comparison-index.ts` (already complete)
- `property-path.ts` (already complete)
- `resultset.ts` (already complete)
- `entity.ts` (no changes required; adapter function handles Filterable gap)

---

## 11. Server-Only Methods (Defer)

The following Rust method is Entity-specific for remote/server subscriptions and should be deferred:

### `upsertQuery(...)` on Reactor (concrete Entity + Attested<Event>)
```rust
pub async fn upsert_query<SE, PA>(...) -> anyhow::Result<Vec<Entity>>
```
Uses `NodeAndContext`, `PolicyAgent::ContextData`, and server-side subscription relay. Defer until peer/connector layer is ported.

### `Subscription.upsertQuery(...)` on Subscription (concrete Entity)
```rust
pub fn upsert_query<SE, PA>(...) -> EntityResultSet<Entity>
```
Server-side idempotent query registration. Defer.
