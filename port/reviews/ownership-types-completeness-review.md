# Ownership Types Completeness Review

**Reviewer**: completeness-reviewer-2
**Date**: 2026-03-15
**Scope**: Verify that every Rust ownership pattern in `core/`, `signals/`, and `proto/` has a corresponding TS type in `@ankurah/base`.

---

## 1. TS Type System Inventory

The `@ankurah/base` package provides:

| TS Type | Rust Equivalent | Location |
|---------|----------------|----------|
| `AkObject` | Base for all ported types (auto-cascade drop glue) | `object.ts` |
| `Drop` | `impl Drop for T` (abstract `drop()`) | `std/drop.ts` |
| `DropGuard` | Composition helper for non-inheriting types | `std/drop.ts` |
| `Arc<T>` | `std::sync::Arc<T>` | `std/arc.ts` |
| `Weak<T>` | `std::sync::Weak<T>` | `std/arc.ts` |
| `Mutex<T>` | `std::sync::Mutex<T>` | `std/sync.ts` |
| `MutexGuard<T>` | `std::sync::MutexGuard<T>` | `std/sync.ts` |
| `RefCell<T>` | `std::cell::RefCell<T>` | `std/cell.ts` |
| `Ref<T>` | `std::cell::Ref<T>` | `std/cell.ts` |
| `RefMut<T>` | `std::cell::RefMut<T>` | `std/cell.ts` |
| `AsyncMutex` | `tokio::sync::Mutex<()>` | `std/async_mutex.ts` |
| `Borrow<T>` | `&T` (non-owning ref, no-op dispose) | `std/borrow.ts` |
| `BorrowMut<T>` | `&mut T` (non-owning ref, no-op dispose) | `std/borrow.ts` |
| `Struct` | Base for ported structs | `struct.ts` |
| `Enum<V>` | Base for ported enums (with `match()` + `is()`) | `enum.ts` |
| `disposeSymbol` | `Symbol.dispose` polyfill | `drop_registry.ts` |
| `leakRegistry` | `FinalizationRegistry` for leak detection | `drop_registry.ts` |

---

## 2. Rust Pattern Coverage Analysis

### 2.1 `impl Drop` -- COVERED

**Rust occurrences found:**
- `core/src/transaction.rs:126` -- `impl Drop for Transaction` (sets `alive` to false)
- `core/src/livequery.rs:284` -- `impl Drop for Inner` (unsubscribes remote predicate)
- `signals/src/` -- no `impl Drop` found (cleanup is via Arc dropping inner)

**TS mapping**: `extends Drop` with abstract `drop()` method. **COMPLETE.**

### 2.2 `Arc<T>` -- COVERED

**Rust occurrences** (heavily used, 60+ sites across core and signals):
- `core/src/entity.rs` -- `Entity(Arc<EntityInner>)`, `TemporaryEntity(Arc<EntityInner>)`
- `core/src/node.rs` -- `Node(Arc<NodeInner<SE, PA>>)`
- `core/src/livequery.rs` -- `EntityLiveQuery(Arc<Inner>)`
- `core/src/reactor.rs` -- `Reactor(Arc<ReactorInner<E, Ev>>)`
- `core/src/resultset.rs` -- `EntityResultSet(Arc<Inner<E>>)`
- `core/src/context.rs` -- `Context(Arc<dyn TContext>)`
- `core/src/system.rs` -- `SystemManager(Arc<Inner<SE, PA>>)`
- `core/src/collectionset.rs` -- `CollectionSet(Arc<Inner<SE>>)`
- `core/src/retrieval.rs` -- `LocalRetriever(Arc<LocalRetrieverInner>)`
- `core/src/storage.rs` -- `StorageCollectionWrapper(Arc<dyn StorageCollection>)`
- `signals/src/broadcast.rs` -- `Broadcast(Arc<Inner<T>>)`
- `signals/src/signal/calculated.rs` -- `Calculated(Arc<Inner<T>>)`
- `signals/src/react_native.rs` -- `ReactObserver(Arc<Inner>)`
- `signals/src/react.rs` -- `ReactObserver(Arc<Inner>)`
- `signals/src/observer/callback_observer.rs` -- `CallbackObserver(Arc<Inner>)`
- `signals/src/value.rs` -- `ValueCell(Arc<RwLock<T>>)`, `ReadValueCell(Arc<RwLock<T>>)`

**TS mapping**: `Arc<T>` class with `clone()`, `drop()`, `downgrade()`, `value` getter, and `[Symbol.dispose]()`. **COMPLETE.**

### 2.3 `Weak<T>` -- COVERED

**Rust occurrences:**
- `core/src/entity.rs:82` -- `WeakEntity(Weak<EntityInner>)`
- `core/src/node.rs:108` -- `WeakNode(Weak<NodeInner<SE, PA>>)`
- `core/src/livequery.rs:58` -- `WeakEntityLiveQuery(Weak<Inner>)`
- `core/src/reactor/fetch_gap.rs:43` -- `weak_node: Weak<NodeInner<SE, PA>>`
- `core/src/property/value/pn_counter.rs:20` -- `backend: Weak<PNBackend>`
- `signals/src/broadcast.rs:59` -- `inner: Weak<Inner<T>>` (in ListenerGuard)
- `signals/src/react_native.rs:42` -- `ReactObserverWeak(Weak<Inner>)`
- `signals/src/react.rs:42` -- `ReactObserverWeak(Weak<Inner>)`
- `signals/src/observer/callback_observer.rs:21` -- `WeakCallbackObserver(Weak<Inner>)`

**TS mapping**: `Weak<T>` class with `upgrade()` returning `Arc<T> | null`, `drop()`. **COMPLETE.**

### 2.4 `Mutex<T>` (std::sync) -- COVERED

**Rust occurrences** (25+ sites):
- `core/src/reactor.rs:70,72` -- subscription and watcher_set mutexes
- `core/src/resultset.rs:64` -- `state: std::sync::Mutex<State<E>>`
- `core/src/retrieval.rs:81,183` -- staged_events
- `core/src/node.rs:152` -- entity_subscription_state
- `core/src/peer_subscription/client_relay.rs:53` -- subscriptions
- `signals/src/react_native.rs:38` -- `trigger_render: Mutex<...>`
- `signals/src/reactive_graph.rs:60` -- bridges

**TS mapping**: `Mutex<T>` with `lock()` returning `MutexGuard<T>`. **COMPLETE.**

### 2.5 `tokio::sync::Mutex` -- COVERED

**Rust occurrences:**
- `core/src/reactor.rs:74` -- `notify_lock: tokio::sync::Mutex<()>`

**TS mapping**: `AsyncMutex` with `acquire()` returning release function. **COMPLETE.**

### 2.6 `RefCell<T>` -- COVERED

**Rust occurrences:**
- `signals/src/context.rs:15` -- `thread_local! { static OBSERVER_STACK: RefCell<Vec<Arc<dyn Observer>>> }`

**TS mapping**: `RefCell<T>` with `borrow()` -> `Ref<T>`, `borrow_mut()` -> `RefMut<T>`. **COMPLETE.**

### 2.7 `RwLock<T>` -- COVERED (via Mutex)

**Rust occurrences** (20+ sites):
- `core/src/entity.rs:53` -- `state: RwLock<EntityInnerState>`
- `core/src/collectionset.rs:22` -- `collections: RwLock<BTreeMap<...>>`
- `core/src/system.rs:35,38,39,42` -- multiple RwLock fields
- `core/src/transaction.rs:33` -- `created_entity_ids: RwLock<HashSet<EntityId>>`
- `signals/src/broadcast.rs:39` -- `listeners: RwLock<HashMap<...>>`
- `signals/src/signal/calculated.rs:26` -- entries
- `signals/src/value.rs:5,8` -- ValueCell/ReadValueCell

**TS mapping**: Per `ownership.md`, `RwLock<T>` maps to `Mutex<T>` (no reader/writer distinction needed in single-threaded JS). **COMPLETE.**

### 2.8 `AtomicBool` / `AtomicUsize` / `AtomicU32` -- COVERED (via plain JS values)

**Rust occurrences** (15+ sites):
- `core/src/transaction.rs:29` -- `alive: Arc<AtomicBool>`
- `core/src/entity.rs:62` -- `trx_alive: Arc<AtomicBool>` in EntityKind
- `core/src/resultset.rs:65` -- `loaded: AtomicBool`
- `core/src/livequery.rs:44,46` -- `AtomicU32` for version tracking
- `signals/src/broadcast.rs:40` -- `next_id: AtomicUsize`
- `signals/src/react_native.rs:35` -- `version: AtomicUsize`
- `signals/src/react.rs:64` -- `version: Arc<AtomicUsize>`

**TS mapping**: Per `ownership.md`, these map to plain `boolean` / `number` in single-threaded JS. **COMPLETE.** No special type needed.

### 2.9 `OnceLock` / `OnceCell` -- NOT PROVIDED (acceptable)

**Rust occurrences:**
- `core/src/system.rs:40` -- `loaded: OnceLock<()>`
- `core/src/task.rs:4` -- `static RUNTIME_HANDLE: OnceLock<Handle>`
- `core/src/peer_subscription/client_relay.rs:57` -- `node: OnceLock<Arc<dyn TNode>>`

**Assessment**: `OnceLock` is a write-once-then-read cell. In JS, this is trivially a nullable field with a setter that throws on second write, or just a `let` variable set once. No dedicated type needed -- idiomatic JS patterns suffice. **ACCEPTABLE GAP.**

### 2.10 `Pin<Box<F>>` -- NOT PROVIDED (acceptable)

**Rust occurrences:**
- `core/src/util/ready_chunks.rs:9` -- `inner: FuturesUnordered<Pin<Box<F>>>`

**Assessment**: `Pin` prevents value movement in memory, which has no JS equivalent or need. Futures/Promises in JS are inherently heap-allocated and never moved. **ACCEPTABLE GAP.**

### 2.11 `Box<dyn T>` -- NOT PROVIDED (acceptable)

**Rust occurrences** (20+ sites): Used for trait objects, error boxing, dynamic dispatch.

**Assessment**: JS uses interfaces and class hierarchies natively. `Box<dyn T>` simply becomes `T` (the interface/abstract type). **ACCEPTABLE GAP.**

### 2.12 `Rc<T>` -- NOT USED

No `Rc<T>` found in core or signals. Not needed. **N/A.**

---

## 3. Enum Pattern Coverage

### 3.1 Variant Shapes

All three Rust enum variant shapes appear in the codebase:

| Shape | Example | TS Enum<V> |
|-------|---------|------------|
| **Unit** | `CausalRelation::Equal` | `{ Equal: {} }` -- uses `{}` for unit |
| **Tuple** (single) | `DecodeError::Other(anyhow::Error)` | `{ Other: { _0: Error } }` -- wrap in object |
| **Tuple** (multi) | `MutationError::FailedStep(&'static str, String)` | `{ FailedStep: { _0: string; _1: string } }` |
| **Struct** | `CausalRelation::DivergedSince { meet, subject, other }` | `{ DivergedSince: { meet: Clock; subject: Clock; other: Clock } }` |

**All variant shapes map cleanly to `Enum<V>`.** The convention of `{}` for unit and `{ _0, _1, ... }` for tuples is sound.

### 3.2 Enum Variants Owning Arc

**Found:**
- `signals/src/broadcast.rs:20-25` -- `BroadcastListener<T>`:
  ```rust
  Payload(Arc<dyn Fn(T) + Send + Sync + 'static>),
  NotifyOnly(Arc<dyn Fn() + Send + Sync + 'static>),
  ```
- `core/src/entity.rs:60-63` -- `EntityKind`:
  ```rust
  Transacted { trx_alive: Arc<AtomicBool>, upstream: Entity },
  ```
  (where `Entity` is itself `Arc<EntityInner>`)

**TS cascade analysis**: The `Enum` class's `[disposeSymbol]()` method iterates over `Object.getOwnPropertyNames(this.value)` and calls `[disposeSymbol]()` on any field that has it. Since `Arc<T>` in TS implements `[disposeSymbol]()`, an enum variant containing an `Arc` field **will** correctly cascade the drop. **COVERED.**

### 3.3 Complex Enum Examples from Proto

All proto enums verified to be expressible with `Enum<V>`:

- `CausalRelation` (5 variants: 2 unit, 3 struct) -- **expressible**
- `DeltaContent` (3 variants: all struct) -- **expressible**
- `NodeRequestBody` (6 variants: all struct/tuple) -- **expressible**
- `NodeResponseBody` (7 variants: mix of unit, tuple, struct) -- **expressible**
- `Message` (2 variants: tuple) -- **expressible**
- `NodeMessage` (6 variants: mix) -- **expressible**
- `NodeUpdateBody` (1 variant: struct) -- **expressible**
- `UpdateContent` (2 variants: tuple) -- **expressible**
- `MembershipChange` (3 variants: all unit) -- **expressible**
- `NodeUpdateAckBody` (2 variants: unit + tuple) -- **expressible**
- `sys::Item` (3 variants: unit + struct + catch-all) -- **expressible** (catch-all via `Other: {}`)
- `DecodeError` (7 variants: mix of unit and tuple) -- **expressible**
- `IdParseError` (3 variants: 2 unit + 1 tuple) -- **expressible**

**All 13+ proto enums are expressible. COMPLETE.**

### 3.4 Core Enums

- `RetrievalError` (15+ variants, many wrapping Box<dyn Error>) -- **expressible**
- `MutationError` (16+ variants) -- **expressible**
- `RequestError` (6 variants) -- **expressible**
- `SubscriptionError` (3 unit variants) -- **expressible**
- `StateError` (3 variants with Box<dyn Error>) -- **expressible**
- `ValidationError` (4 variants) -- **expressible**
- `ApplyError` (4 variants including `Items(Vec<...>)`) -- **expressible**
- `LineageError` (3 variants: unit + struct) -- **expressible**
- `ItemChange<I>` (4 variants: struct with generic) -- **expressible** (generic Enum)
- `ChangeKind` (4 unit variants) -- **expressible**
- `EntityKind` (2 variants: unit + struct with Arc) -- **expressible**
- `lineage::Ordering<Id>` (4 variants: unit + struct) -- **expressible**
- `WatcherOp` (2 unit variants) -- **expressible**
- `WatcherChange` (2 struct variants) -- **expressible**
- `EntityWatcherId` (2 tuple variants) -- **expressible**
- `resultset::IVec` (2 variants: fixed array + Vec) -- **expressible**

**All core enums expressible. COMPLETE.**

### 3.5 Signals Enums

- `BroadcastListener<T>` (2 variants owning Arc<dyn Fn>) -- **expressible** (Arc fields cascade correctly)

**COMPLETE.**

---

## 4. Match Pattern Coverage

### 4.1 Exhaustive Match -- COVERED

The `match<R>(arms)` method requires all keys of `V` to be present, enforced by the TypeScript mapped type `{ [K in keyof V]: (value: V[K]) => R }`. Missing an arm is a compile-time TS error. **COMPLETE.**

### 4.2 `if let` -- COVERED via `is()`

Rust `if let Some(x) = val { ... }` or `if let MyEnum::Variant(x) = val { ... }` maps to `is()`:
```typescript
if (val.is('Variant')) {
    // val.value is narrowed to V['Variant']
}
```
**COMPLETE.**

### 4.3 Wildcard `_ =>` -- EXPRESSIBLE with workaround

Rust's `_ =>` catch-all arm has no direct TS equivalent in the `match()` API since it requires exhaustive arms. However, this is **acceptable** because:
1. Non-exhaustive match in TS can use `if/else if` chains with `is()`.
2. A `_` arm can be simulated by providing a function for every remaining variant that delegates to a default.
3. Exhaustiveness is actually *safer* -- it catches missing cases at compile time.

**ACCEPTABLE -- no change needed.**

### 4.4 Nested Match -- EXPRESSIBLE

Rust nested `match` (40 files with nested matches found) is expressed by nesting `match()` calls in TS:
```typescript
outer.match({
    Variant: (v) => inner.match({
        SubVariant: (sv) => /* ... */,
        // ...
    }),
    // ...
});
```
**COMPLETE.**

### 4.5 `while let` -- EXPRESSIBLE

`while let` on enums maps to `while (val.is('Variant')) { ... }` loops in TS. **COMPLETE.**

---

## 5. Spec vs Implementation Discrepancies

**IMPORTANT**: The `port/ownership.md` spec and `port/ownership/provided-types.md` spec use different naming than the actual implementation:

| Spec Names | Implementation Names | Notes |
|-----------|---------------------|-------|
| `Disposable` | `Drop` (extends `AkObject`) | Spec says `Disposable` + `onDispose()`, impl uses `Drop` + `drop()` |
| `DisposeGuard` | `DropGuard` | Same pattern, different name |
| `dispose()` / `onDispose()` | `drop()` / `[disposeSymbol]()` | Method naming diverged |
| Arc -> `T` (delete wrapper) | `Arc<T>` class kept | Spec said to delete Arc, impl preserves it |
| Weak -> `WeakRef<T>` | `Weak<T>` class | Spec said native WeakRef, impl provides custom class |
| N/A | `AkObject` | Not in spec; base class with auto-cascade drop glue |
| N/A | `Struct` | Not in spec; trivial base class |
| N/A | `Enum<V>` | Not in spec; enum base with match/is |
| N/A | `Borrow<T>` / `BorrowMut<T>` | Not in spec; non-owning reference wrappers |

The implementation is **richer and more faithful** to Rust than the spec suggests. The spec appears to be an older design doc that was superseded by the actual implementation. The implementation's approach (keeping `Arc<T>`, `Weak<T>`, `Borrow<T>`, `BorrowMut<T>`) is correct for preserving 1:1 translation fidelity.

---

## 6. Patterns NOT Expressible (True Gaps)

### 6.1 No `RwLock<T>` Distinct Type -- MINOR GAP

`RwLock` is mapped to `Mutex` per spec, which is correct semantically. However, for 1:1 translation fidelity, having a `RwLock<T>` type alias or thin wrapper would allow translated code to use `rwlock.read()` / `rwlock.write()` instead of `mutex.lock()`, keeping the TS closer to Rust source.

**Severity**: Low. The semantics are correct; only surface-level API naming differs.

**Recommendation**: Consider adding `RwLock<T>` as a type alias or thin wrapper over `Mutex<T>` with `read()` -> `Ref<T>` and `write()` -> `RefMut<T>` methods.

### 6.2 No `thread_local!` Equivalent -- MINOR GAP

`signals/src/context.rs` uses `thread_local! { static OBSERVER_STACK: RefCell<Vec<...>> }`. In JS, module-level variables serve the same purpose (single-threaded). No dedicated type needed, but there's no documented mapping rule for this pattern.

**Severity**: Negligible. Module-scope variables are the natural equivalent.

### 6.3 `#[serde(other)]` Catch-All Enum Variant -- MINOR GAP

`proto/src/sys.rs` uses `#[serde(other)] Other` for forward-compatible deserialization. The `Enum<V>` type can represent this variant, but the deserialization layer (bincode) needs to handle unknown discriminants gracefully.

**Severity**: Low. This is a serialization concern, not an ownership type concern.

---

## 7. Summary Verdict

| Category | Status |
|----------|--------|
| `impl Drop` | COMPLETE |
| `Arc<T>` | COMPLETE |
| `Weak<T>` | COMPLETE |
| `Mutex<T>` (std::sync) | COMPLETE |
| `MutexGuard<T>` | COMPLETE |
| `RefCell<T>` / `Ref<T>` / `RefMut<T>` | COMPLETE |
| `tokio::sync::Mutex` | COMPLETE |
| `RwLock<T>` | COMPLETE (via Mutex) |
| Atomics | COMPLETE (via plain values) |
| `OnceLock` / `Pin` / `Box<dyn>` | ACCEPTABLE GAP (no TS type needed) |
| `Borrow<T>` / `BorrowMut<T>` | COMPLETE |
| `Enum<V>` (all variant shapes) | COMPLETE |
| Enum owning Arc (cascade) | COMPLETE |
| `match()` (exhaustive) | COMPLETE |
| `is()` (if-let equivalent) | COMPLETE |
| Nested match | COMPLETE |
| Wildcard `_` arm | ACCEPTABLE (use if/else chain) |
| Auto-cascade drop glue | COMPLETE (AkObject) |
| Leak detection | COMPLETE (FinalizationRegistry) |
| **Spec-vs-impl naming** | **DIVERGED** (impl is better) |

**Overall**: The `@ankurah/base` ownership type system is **comprehensive and complete** for all Rust ownership patterns found in the ankurah codebase. Every `impl Drop`, `Arc`, `Weak`, `Mutex`, `RwLock`, `RefCell`, and `tokio::sync::Mutex` pattern has a corresponding TS type. All enum variant shapes (unit, tuple, struct) and enum-owns-Arc patterns are expressible. All match patterns (exhaustive, if-let, nested) are covered.

The only actionable item is updating `port/ownership.md` and `port/ownership/provided-types.md` to reflect the actual implementation naming (`Drop`/`AkObject` instead of `Disposable`, preserved `Arc<T>`/`Weak<T>` instead of deletion).
