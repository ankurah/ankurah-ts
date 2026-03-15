# Async Safety Review: memory-model.md Section 13

**Reviewer**: Async Safety Reviewer Agent
**Date**: 2026-03-12
**Spec under review**: `/ankurah-ts/specs/memory-model.md` (Section 13: Async Serialization / PromiseMutex)
**Verdict**: **PASS WITH NOTES**

---

## 1. Executive Summary

Section 13 of memory-model.md correctly identifies the critical distinction between `std::sync::Mutex` (eliminable in single-threaded JS) and `tokio::sync::Mutex` (requires PromiseMutex in TS). The existing PromiseMutex on the reactor's `notifyLock` is correct. However, the "Required PromiseMutex Coverage" table is **incomplete** -- it is missing several entries where fire-and-forget async patterns or concurrent async operations can cause bugs. The table also has one entry that overstates the risk (SystemManager lifecycle ops use `std::sync::RwLock`, not `tokio::sync::Mutex`, and are adequately protected by single-threaded JS for the synchronous portions).

---

## 2. Per-File Async Function Analysis

### 2.1 `reactor/index.ts` — Reactor

#### `notifyChange(changes)` (line 432)
- **Await points**: `this.notifyLock.acquire()`, `Promise.all(evaluations)`
- **Shared state across awaits**: `this.subscriptions` (Map), `this.watcherSet` (WatcherSet)
- **Rust protection**: `tokio::sync::Mutex<()>` on `notify_lock`; `std::sync::Mutex` on `subscriptions` and `watcher_set` (acquired/released within sync blocks, not held across `.await`)
- **TS protection**: `PromiseMutex` on `notifyLock` -- **CORRECT**
- **Assessment**: Properly serialized. The PromiseMutex ensures only one `notifyChange` runs at a time, matching the Rust `tokio::sync::Mutex<()>` semantics.

#### `addQueryAndNotify(...)` (line 321)
- **Await points**: `node.fetchEntitiesFromLocal(...)` (line 338), `subscription.fillGapsForQuery(...)` (line 356)
- **Shared state across awaits**: `this.subscriptions` (read before await, used after), `subscription` (retrieved before fetch, used after), `this.watcherSet` (mutated by `updateQuery` and `fillGapsForQuery`)
- **Rust protection**: `std::sync::Mutex` on `subscriptions` (acquired, cloned subscription, dropped before `.await`). No `tokio::sync::Mutex` -- relies on subscription being Arc-cloned.
- **TS protection**: None explicit, but single-threaded JS ensures the synchronous code between awaits is atomic. The subscription is looked up, then fetch happens, then mutation happens. Another call to `addQueryAndNotify` for the same subscription could interleave, but in Rust this would also be possible (no `notify_lock` protects this path).
- **Assessment**: **Acceptable**. Matches Rust behavior. The subscription lookup happens before the await, so the subscription reference is stable. A concurrent unsubscribe could remove it from the map, but the subscription object is already captured.

#### `updateQueryAndNotify(...)` (line 379)
- Same pattern as `addQueryAndNotify`. **Acceptable** for the same reasons.

### 2.2 `reactor/subscription_state.ts` — Subscription

#### `evaluateChanges(candidates)` (line 429)
- **Await points**: None in the evaluation phase itself. But at line 532, `this.fillGapsAndNotify(updateItems, gapsToFill)` is called **without `await`** -- this is a fire-and-forget async call.
- **Shared state at risk**: `this._queries` (read/written by `fillGapsAndNotify` via `addEntityWatchers`), `this._watcherSet` (mutated by `addEntityWatchers`)
- **Rust protection**: In Rust, `crate::task::spawn(self.clone().fill_gaps_and_notify(...))` spawns a background task. The task acquires `self.state.lock()` (via `add_entity_watchers`) and `self.watcher_set.lock()`. These `std::sync::Mutex` locks protect against concurrent access from the spawned task.
- **TS protection**: **MISSING**. The fire-and-forget `fillGapsAndNotify()` runs asynchronously and mutates `this._watcherSet` (via `addEntityWatchers`) outside the `notifyLock`. A concurrent `notifyChange` could be reading `watcherSet` in Phase 1 while `fillGapsAndNotify` is writing to it in the background.
- **Assessment**: **This is correctly identified in the spec's table as MISSING.** The spec says: "fire-and-forget `fillGapsAndNotify()` mutates WatcherSet outside the notifyLock."

#### `fillGapsAndNotify(items, gapsToFill)` (line 696)
- **Await points**: `Promise.all(gapFillPromises)` (line 710)
- **Shared state across awaits**: `this._watcherSet` (mutated by `addEntityWatchers` after the await), `items` array (mutated after await)
- **Rust protection**: `std::sync::Mutex` on `watcher_set` (acquired inside `add_entity_watchers`)
- **TS protection**: **MISSING**. This is the fire-and-forget task from `evaluateChanges`. It mutates `_watcherSet` without any serialization.
- **Assessment**: **BUG RISK**. The scenario: `notifyChange` acquires `notifyLock`, spawns `evaluateChanges` for each subscription, `evaluateChanges` completes synchronously and returns `watcherChanges`, but also fires off `fillGapsAndNotify`. Then `notifyChange` applies watcher changes in Phase 3 and releases the lock. Meanwhile, `fillGapsAndNotify` completes its gap fetch and calls `addEntityWatchers`, which mutates `_watcherSet` concurrently with a new `notifyChange` invocation that is reading `_watcherSet` in Phase 1.

#### `fillGapsForQuery(queryId, reactorUpdates)` (line 631)
- **Await points**: `Subscription.processGapFillEntities(gapData)` (line 643)
- **Shared state across awaits**: `this._queries` (read before await), `this._watcherSet` (mutated after await via `addEntityWatchers`)
- **Rust protection**: `std::sync::Mutex` on state (acquired, gap data extracted, dropped before `.await`)
- **TS protection**: Single-threaded JS protects the synchronous portions. The query state is read synchronously, then gap fill happens, then watchers are updated synchronously.
- **Assessment**: **Acceptable** when called from within `addQueryAndNotify`/`updateQueryAndNotify` (which don't hold the notifyLock). However, see the gap-fill-during-notify concern below.

#### `fillGapsForQueryEntities(queryId, entities)` (line 604)
- Same pattern as `fillGapsForQuery`. **Acceptable** with the same caveats.

### 2.3 `livequery.ts` — EntityLiveQuery

#### `activate(version)` (line 344)
- **Await points**: `reactor.addQueryAndNotify(...)` or `reactor.updateQueryAndNotify(...)` (lines 369-390)
- **Shared state across awaits**: `this._selection` (read before await at line 347), `this.initializedVersion` (read before await at line 361)
- **Rust protection**: `AtomicU32` for `initialized_version` and `current_version`. `Mut<(Selection, u32)>` for selection. No `tokio::sync::Mutex` here.
- **TS protection**: Plain number fields for `initializedVersion` and `currentVersion`. `Mut` signal for `_selection`.
- **Assessment**: **Correctly identified as a known issue** (issue #146). Two concurrent `activate` calls (from `create` + `subscriptionEstablished`, or from rapid `updateSelection` calls) can race. The stale version check at line 350 provides some protection, but concurrent activation could still cause version regression if two activations run simultaneously and the newer one completes first.
- **Spec status**: Listed in Section 13 table as "Needs PromiseMutex or activation queue."

#### Fire-and-forget in `EntityLiveQuery.create()` (line 226)
- `void me.activate(1).then(...)` -- fire-and-forget async task
- **Shared state at risk**: The entire EntityLiveQuery (selection, initializedVersion, reactor state)
- **Rust protection**: `crate::task::spawn(async move { me2.activate(1).await })` -- same fire-and-forget pattern. Rust relies on the reactor's internal locks + `notifyLock` for safety.
- **TS protection**: Matches Rust pattern. The `activate` call goes through `reactor.addQueryAndNotify`, which does not acquire `notifyLock` (by design -- it's the initialization path, not the change notification path).
- **Assessment**: **Acceptable** -- matches Rust. The activation is inherently racy with respect to incoming changes, but this is a known design decision (same in Rust).

#### Fire-and-forget in `updateSelection()` (line 316)
- `void this.activate(newVersion).then(...)` -- fire-and-forget
- Same analysis as above. **Acceptable** -- matches Rust.

#### `waitInitialized()` (line 265)
- **Await points**: `this._initPromise` (line 271)
- **Shared state across awaits**: `this.initializedVersion`, `this.currentVersion`
- **Assessment**: No mutation of shared state across the await. **Safe**.

#### `updateSelectionWait(newSelection)` (line 332)
- Calls `updateSelection()` (sync) then `await waitInitialized()`. No additional shared state concerns.

### 2.4 `node.ts` — NodeAndContext

#### `commitLocalTrx(trx)` (line 260)
- **Await points**: Multiple: `this.node.storageEngine.collection(...)` (lines 316, 325), `collection.addEvent(...)` (line 317), `collection.setState(...)` (line 347), `this.node.reactor.notifyChange(...)` (line 357)
- **Shared state across awaits**: `trx.alive` (set false at line 265 before any await -- safe), `this.node.entities` (WeakEntitySet, read/written at lines 128-133 in `fetchEntitiesFromLocal`), `this.node.reactor` (called at line 357)
- **Rust protection**: `trx.alive` uses `Arc<AtomicBool>` (atomic swap). Entity operations use their own internal locks. `reactor.notifyChange` uses `tokio::sync::Mutex<()>`.
- **TS protection**: `trx.alive.value = false` is synchronous (before any await). Reactor `notifyChange` uses `PromiseMutex`. Entity operations are single-threaded.
- **Assessment**: **Safe**. The alive flag is set before any await, matching Rust's atomic swap. Concurrent commits are prevented by the alive flag check-and-set happening synchronously (line 262-265). Two concurrent commits on the same transaction: the first sets `alive = false`, the second throws at line 263.

#### `getEntity(id, collection, cached)` (line 195)
- **Await points**: `this.node.storageEngine.collection(...)`, `storageCollection.getState(...)`
- **Shared state across awaits**: `this.node.entities` (checked before await, used after)
- **Assessment**: The local check at line 197 is a cache optimization. After the await, `this.node.entities.withState(...)` is called, which handles deduplication internally (creates or returns existing). **Safe** -- matches Rust pattern.

#### `fetchEntities(collection, args)` (line 227)
- Delegates to `this.node.fetchEntitiesFromLocal(...)`. **Safe**.

### 2.5 `node.ts` — Node

#### `fetchEntitiesFromLocal(collectionId, selection)` (line 123)
- **Await points**: `this.storageEngine.collection(...)`, `collection.fetchStates(...)`
- **Shared state across awaits**: `this.entities` (WeakEntitySet)
- **Rust protection**: `std::sync::Mutex` on the inner HashMap of WeakEntitySet.
- **TS protection**: Single-threaded JS. The `withState` calls happen synchronously after the await.
- **Assessment**: **Safe**. Each `withState` call is synchronous and cannot be interleaved.

### 2.6 `system.ts` — SystemManager

#### `loadSystemCatalog()` (line 390)
- **Await points**: `this.collectionset.get(...)`, `storage.fetchStates(...)`
- **Shared state across awaits**: `this._items`, `this._root`, `this.loaded`, `this.systemReady`
- **Rust protection**: `RwLock` on `items`, `root`, `system_ready`. `OnceLock` on `loaded`.
- **TS protection**: Plain fields. Single-threaded JS protects synchronous access.
- **Assessment**: This is called once from the constructor as a fire-and-forget. The `loaded` flag uses `OnceLock` in Rust; TS uses a boolean. Since this runs exactly once (checked at line 391-393), and all field writes happen synchronously between awaits, this is **safe in practice**. However, if `create()` or `joinSystem()` are called before `loadSystemCatalog()` completes, they `await this.waitLoaded()` which blocks until loading is done. **Safe**.

#### `create()` (line 244)
- **Await points**: `this.waitLoaded()`, `this.collectionset.get(...)`, `storage.addEvent(...)`, `storage.setState(...)`
- **Shared state across awaits**: `this._items`, `this._root`, `this.systemReady`
- **Rust protection**: `RwLock` on each field (acquired briefly for writes, never held across `.await`)
- **TS protection**: Plain fields. All field writes happen synchronously between awaits.
- **Assessment**: **The spec flags this as MISSING.** However, I assess the actual risk differently:
  - In Rust, the `RwLock` guards are acquired and dropped within synchronous blocks, never held across `.await`. They protect against concurrent access from other threads/tasks.
  - In TS, since JS is single-threaded, the synchronous field writes between await points are inherently atomic.
  - The real risk: two concurrent `create()` calls could both pass the `_items.length === 0` check (line 252) before either writes `_items`. But in practice, both would `await waitLoaded()` first, then both would check synchronously. The first one modifies `_items` synchronously, the second would see it non-empty and throw.
  - **Verdict**: The risk is **LOW** because the check-and-mutate (lines 252-285) happens entirely within a single synchronous block between `await waitLoaded()` and `await storage.addEvent(...)`. No interleaving is possible until the first `await` after line 258. By that point, `_items.push(systemEntity)` has already happened (at line 285... wait, no -- the push happens at line 285, AFTER the awaits at lines 274 and 282).
  - **Corrected**: Actually `_items.push(systemEntity)` at line 285 happens AFTER `await storage.addEvent(attestedEvent)` (line 274) and `await storage.setState(attestedState)` (line 282). So between lines 258 and 285, there are two await points. A concurrent `create()` call could interleave and also pass the `_items.length === 0` check.
  - **Conclusion**: **The spec is correct** -- this is a real race condition. A PromiseMutex or a synchronous "claimed" flag set before the first await would fix it.

#### `joinSystem(state)` (line 301)
- **Await points**: `this.waitLoaded()`, `this.hardReset()`, `this.collectionset.get(...)`, `storage.setState(...)`
- **Shared state across awaits**: `this._root`, `this.systemReady`, `this._items`
- **Rust protection**: `RwLock` on each field.
- **TS protection**: None.
- **Assessment**: Same issue as `create()`. Two concurrent `joinSystem()` calls could race through the root-state check and both proceed. In Rust, the `RwLock` prevents two writers from seeing stale state, but since locks aren't held across `.await`, even Rust has a TOCTOU window here (check root, then await, then modify). The Rust code drops locks before `.await`, so the same race exists in Rust. **The spec is slightly overstated here** -- the race exists in both Rust and TS. A `tokio::sync::Mutex` would be needed in Rust to truly serialize, and a PromiseMutex in TS.

#### `hardReset()` (line 365)
- **Await points**: None (all synchronous in TS, `collectionset.deleteAllCollections()` is sync)
- **Assessment**: **Safe** in TS. In Rust, `delete_all_collections().await` is async, but in TS it's synchronous (as noted in the comment at line 367).

### 2.7 `node_applier.ts` — NodeApplier

#### `applyUpdates(node, fromPeerId, items)` (line 48)
- **Await points**: `NodeApplier.applyUpdate(...)` in a loop (line 79), `node.reactor.notifyChange(changes)` (line 82)
- **Shared state across awaits**: `changes` array (local, no concern), `node.entities` (WeakEntitySet), `node.reactor`
- **Rust protection**: Each `applyUpdate` does storage I/O. `notify_change` uses `tokio::sync::Mutex<()>`.
- **TS protection**: `notifyChange` uses `PromiseMutex`.
- **Assessment**: **Safe**. The changes array is local. `notifyChange` is properly serialized.

#### `applyDeltas(node, fromPeerId, deltas, retriever)` (line 92)
- **Await points**: `ReadyChunks` async iteration, `node.reactor.notifyChange(batch)` per chunk
- **Shared state across awaits**: `node.entities` (mutated by each `applyDelta`), `node.reactor`
- **Rust protection**: Same as above. Entity operations have their own locks.
- **TS protection**: `notifyChange` uses `PromiseMutex`. Entity operations are single-threaded.
- **Assessment**: **Safe** for reactor notification. However, multiple `applyDelta` calls run concurrently via `Promise.all`-like semantics in `ReadyChunks`. Each one calls `node.entities.withState(...)` which could interleave at await points. In Rust, `WeakEntitySet` uses `std::sync::Mutex` internally. In TS, `WeakEntitySet` is a plain Map. However, since each `applyDelta` awaits on storage I/O and the `withState` calls happen synchronously, there's no actual interleaving risk for the entity set -- each `withState` completes synchronously between awaits. **Safe**.

#### `applyUpdate(...)`, `applyDelta(...)`, `applyDeltaInner(...)` (private helpers)
- All follow the same pattern: storage I/O (await), then synchronous entity mutation. **Safe**.

### 2.8 `transaction.ts` — Transaction

#### `create(model, values)` (line 112)
- **Await points**: None currently (the method is declared `async` but has no `await` -- all operations are synchronous).
- **Assessment**: **Safe** (no interleaving possible). The `async` is for API compatibility with Rust which does `await` for storage retrieval.

#### `get(model, id)` (line 139)
- **Await points**: `this.dyncontext.getEntity(...)` (line 150)
- **Shared state across awaits**: `this.entities` (checked before await at line 144, re-checked after at line 153)
- **Rust protection**: `AppendOnlyVec` (lock-free, but append-only ensures no data races). The race check pattern (lines 152-155) is identical in Rust.
- **TS protection**: Plain array + race check pattern.
- **Assessment**: **Safe**. The race check at lines 153-155 handles the case where another `get()` call for the same entity completes between our check and our fetch. This matches Rust exactly.

#### `commit()` (line 203)
- Delegates to `this.dyncontext.commitLocalTrx(this)`. See NodeAndContext analysis above.

---

## 3. Assessment of Section 13 "Required PromiseMutex Coverage" Table

### Entry 1: Reactor notification pipeline
- **Status**: Already implemented. **CORRECT**.
- The `notifyLock` PromiseMutex in `reactor/index.ts` correctly mirrors `tokio::sync::Mutex<()>` from `reactor.rs:74`.

### Entry 2: WatcherSet mutation from gap-fill
- **Status**: Correctly identified as **MISSING**.
- **Analysis**: The fire-and-forget `fillGapsAndNotify()` at `subscription_state.ts:532` runs outside the `notifyLock`. In Rust, the background task acquires `std::sync::Mutex` on `watcher_set` before mutating it. In TS, there is no lock -- `addEntityWatchers` directly mutates `this._watcherSet`.
- **Interleaving scenario**:
  1. `notifyChange` A acquires `notifyLock`, runs Phase 1 (reads `watcherSet`), spawns `evaluateChanges` for each subscription
  2. `evaluateChanges` returns `watcherChanges` and fires off `fillGapsAndNotify` (no await)
  3. `notifyChange` A applies watcher changes (Phase 3), releases `notifyLock`
  4. `notifyChange` B acquires `notifyLock`, starts Phase 1 (reads `watcherSet`)
  5. `fillGapsAndNotify` from step 2 completes its gap fetch and calls `addEntityWatchers`, mutating `watcherSet` **while `notifyChange` B is reading it in Phase 1**
- In single-threaded JS, step 5 cannot truly run *simultaneously* with step 4. But the mutations from `fillGapsAndNotify` happen outside the `notifyLock` serialization, so `notifyChange` B's Phase 1 snapshot of `watcherSet` could be stale (missing the gap-filled entity watchers). This means the gap-filled entities might not receive change notifications until the *next* `notifyChange` cycle.
- **Suggested fix**: Either (a) await gap fill within `evaluateChanges` (under the `notifyLock`), or (b) add a WatcherSet-level PromiseMutex for mutations, or (c) make `fillGapsAndNotify` queue its watcher mutations to be applied in the next `notifyChange` Phase 3.

### Entry 3: SystemManager lifecycle ops
- **Status**: Identified as **MISSING** in spec. **Partially correct**.
- **Analysis**: The Rust code uses `RwLock` (not `tokio::sync::Mutex`) on `_root`, `system_ready`, `_items`. These `RwLock` guards are acquired and dropped within synchronous blocks, never held across `.await`. They exist for thread safety (multi-threaded tokio runtime), not async serialization.
- However, the TOCTOU race in `create()` is real: `_items.length === 0` is checked at line 252, but `_items.push(...)` happens at line 285, with two `await` points in between (lines 274, 282). A concurrent `create()` could pass the check while the first one is awaiting storage.
- The same race exists in Rust (locks are dropped before `.await`), but it's less likely in practice because `create()` is typically called once during initialization.
- **Conclusion**: The spec is correct that serialization is needed, though the risk is low (initialization-time only). A simple synchronous "creating" boolean flag set before the first `await` would be simpler than a full PromiseMutex.

### Entry 4: LiveQuery activation ordering
- **Status**: Correctly identified. **No Rust protection either** (issue #146).
- **Analysis**: Two concurrent `activate()` calls (e.g., from `create()` fire-and-forget + a rapid `updateSelection()`) could both proceed through the stale-version check and call `reactor.addQueryAndNotify` / `reactor.updateQueryAndNotify`. The first activation would call `addQueryAndNotify` (which registers the query), the second would also try `addQueryAndNotify` (because `initializedVersion` is still 0) and would get an error because the query already exists.
- **Suggested fix**: PromiseMutex on `activate()` within EntityLiveQuery, or an activation queue.

---

## 4. MISSING Entries in the Table

### MISSING 1: `commitLocalTrx` storage-then-mutate race (LOW RISK)

| Field | Value |
|-------|-------|
| **File** | `node.ts` (NodeAndContext) |
| **Function** | `commitLocalTrx()` |
| **Shared state at risk** | Upstream entity state (via `upstream.applyEvent()` at line 331) |
| **Rust protection** | Entity internal locks (`std::sync::Mutex` on backends) |
| **TS protection** | Single-threaded JS (synchronous entity mutation between awaits) |
| **Risk** | LOW -- entity mutations are synchronous between awaits |
| **Suggested TS fix** | None needed -- single-threaded JS provides equivalent protection |

This is correctly NOT in the table because it doesn't need a PromiseMutex.

### MISSING 2: `NodeApplier.applyDeltas` concurrent entity mutation (LOW RISK)

| Field | Value |
|-------|-------|
| **File** | `node_applier.ts` |
| **Function** | `applyDeltas()` |
| **Shared state at risk** | `node.entities` (WeakEntitySet) mutated by concurrent `applyDelta` calls |
| **Rust protection** | `std::sync::Mutex` on WeakEntitySet internals |
| **TS protection** | None explicit, but `withState` is synchronous |
| **Risk** | LOW -- each `withState` call completes synchronously between awaits |
| **Suggested TS fix** | None needed for single-threaded JS |

This is also correctly NOT in the table.

### MISSING 3: `Subscription.evaluateChanges` fire-and-forget watcher mutation (already covered by Entry 2)

This is already covered by the WatcherSet entry, but the table should be more specific about the mechanism: `evaluateChanges()` at line 532 calls `this.fillGapsAndNotify(...)` without `await`, creating a detached async task that later mutates `_watcherSet`.

---

## 5. Detailed Assessment of Fire-and-Forget Patterns

The user feedback specifically called out fire-and-forget async tasks as the most dangerous pattern. Here is a complete inventory:

### 5.1 `EntityLiveQuery.create()` — `void me.activate(1).then(...)`
- **Location**: `livequery.ts:226`
- **Rust equivalent**: `crate::task::spawn(async move { me2.activate(1).await })`
- **Risk**: MEDIUM -- concurrent activation with incoming `subscriptionEstablished` (but same risk in Rust)
- **Mitigation needed**: PromiseMutex on activate (per spec entry 4)

### 5.2 `EntityLiveQuery.updateSelection()` — `void this.activate(newVersion).then(...)`
- **Location**: `livequery.ts:316`
- **Rust equivalent**: `crate::task::spawn(async move { me2.activate(new_version).await })`
- **Risk**: MEDIUM -- same as above
- **Mitigation needed**: Same PromiseMutex as 5.1

### 5.3 `Subscription.evaluateChanges()` — `this.fillGapsAndNotify(updateItems, gapsToFill)`
- **Location**: `subscription_state.ts:532`
- **Rust equivalent**: `crate::task::spawn(self.clone().fill_gaps_and_notify(...))`
- **Risk**: **HIGH** -- mutates `_watcherSet` outside `notifyLock`
- **Mitigation needed**: Per spec entry 2 -- must serialize with notifyLock or add WatcherSet-level mutex

### 5.4 `SystemManager constructor` — `this.loadSystemCatalog().catch(...)`
- **Location**: `system.ts:145`
- **Rust equivalent**: `crate::task::spawn(async move { me.load_system_catalog().await })`
- **Risk**: LOW -- runs once, all consumers await `waitLoaded()`
- **Mitigation needed**: None

---

## 6. `std::sync::Mutex` Elimination Correctness

The spec states that `std::sync::Mutex` can be eliminated when:
1. The lock is never held across an `.await` point, AND
2. All accesses are within synchronous code blocks

### Verification of eliminations in the TS codebase:

| Rust `std::sync::Mutex` | Location | Held across `.await`? | TS replacement | Correct? |
|---|---|---|---|---|
| `ReactorInner.subscriptions` | `reactor.rs:70` | No (cloned before await) | Plain `Map` | YES |
| `ReactorInner.watcher_set` | `reactor.rs:72` | No (accessed in sync blocks) | Plain `WatcherSet` | YES* |
| `Inner.state` (Subscription) | `subscription_state.rs:90` | No (dropped before await in `evaluate_changes` line 473) | Plain fields | YES* |

*With the caveat that the fire-and-forget `fillGapsAndNotify` creates a de facto cross-await mutation pattern in TS. In Rust, the `std::sync::Mutex` on `watcher_set` protects the spawned task from concurrent access. In TS, this protection is lost. This is the core issue in Entry 2.

### SystemManager field locks:

| Rust `RwLock` | Location | Held across `.await`? | TS replacement | Correct? |
|---|---|---|---|---|
| `Inner.root` | `system.rs:38` | No | Plain field | YES |
| `Inner.items` | `system.rs:39` | No | Plain field | YES |
| `Inner.system_ready` | `system.rs:42` | No | Plain field | YES |
| `Inner.collection_map` | `system.rs:35` | No | Plain field | YES |

All `RwLock` eliminations are correct. The locks are acquired and released within synchronous blocks.

---

## 7. Recommendations

### 7.1 High Priority (Entry 2 — WatcherSet gap-fill race)
The `fillGapsAndNotify` fire-and-forget is the most dangerous pattern. Recommended fix: make `evaluateChanges` await the gap fill so it completes under the `notifyLock`. This would change:

```typescript
// CURRENT (fire-and-forget):
if (gapsToFill.length > 0) {
    this.fillGapsAndNotify(updateItems, gapsToFill);
} else if (updateItems.length > 0) {
    this._broadcast.send({ items: updateItems });
}

// PROPOSED (awaited):
if (gapsToFill.length > 0) {
    await this.fillGapsAndNotify(updateItems, gapsToFill);
} else if (updateItems.length > 0) {
    this._broadcast.send({ items: updateItems });
}
```

This requires `evaluateChanges` callers to await it, which they already do (via `Promise.all(evaluations)` in `notifyChange`). The gap fill would then complete under the `notifyLock`, ensuring watcher mutations are serialized.

**Trade-off**: This delays notification until gap fill completes, which could add latency. But correctness > performance here.

### 7.2 Medium Priority (Entry 4 — LiveQuery activation serialization)
Add a PromiseMutex to `EntityLiveQuery.activate()`:

```typescript
private activateLock = new PromiseMutex();

private async activate(version: number): Promise<void> {
    return this.activateLock.run(async () => {
        // existing activate logic
    });
}
```

### 7.3 Low Priority (Entry 3 — SystemManager lifecycle)
Add a synchronous "creating" flag to prevent concurrent `create()`:

```typescript
private creating = false;

async create(): Promise<void> {
    if (this.creating) throw new Error('System creation already in progress');
    this.creating = true;
    try {
        // existing create logic
    } finally {
        this.creating = false;
    }
}
```

Or use a PromiseMutex to serialize `create()`, `joinSystem()`, and `hardReset()`.

---

## 8. Conclusion

The spec's Section 13 is **substantially correct** in its analysis. The distinction between `std::sync::Mutex` (eliminable) and `tokio::sync::Mutex` (needs PromiseMutex) is properly articulated. The "Required PromiseMutex Coverage" table identifies the three most important gaps. The primary concern is:

1. **Entry 2 (WatcherSet gap-fill race)** is the highest-risk issue and should be addressed before the TS port is considered production-ready. The fire-and-forget `fillGapsAndNotify` pattern is exactly the dangerous pattern the user flagged.

2. **Entry 4 (LiveQuery activation)** has the same bug in Rust (issue #146), so it's a known design debt, not a TS-specific regression.

3. **Entry 3 (SystemManager)** is low risk in practice (initialization-time only) and the Rust code has the same TOCTOU window.

No entries are fundamentally wrong. The table could benefit from the additional detail provided in this review about specific interleaving scenarios and the severity ranking.

**Verdict: PASS WITH NOTES** -- the spec correctly identifies the key async safety concerns and the table is substantially complete, with the notes above providing additional precision on risk levels and specific interleaving scenarios.
