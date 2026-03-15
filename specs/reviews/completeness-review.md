# Completeness Review: memory-model.md

**Reviewer**: Completeness Reviewer Agent
**Date**: 2026-03-12
**Spec version reviewed**: 2026-03-12 (current)

---

## Verdict: PASS WITH NOTES

The spec covers all significant ownership, Drop, Arc, Weak, Mutex, RwLock, RefCell, and lifetime patterns in the codebase. Two minor gaps exist (both are edge cases that do not affect correctness of the TS port's primary lifecycle management), and two patterns deserve slightly expanded treatment. No critical omissions were found.

---

## 1. Complete Inventory of `impl Drop` Patterns

| # | Type | File | Drop behavior | Spec coverage |
|---|------|------|---------------|---------------|
| 1 | `Transaction` | `core/src/transaction.rs:126` | Sets `alive = false` | Section 3a, Section 10, Section 14 |
| 2 | `ResultSetWrite<'a, E>` | `core/src/resultset.rs:335` | Broadcasts changes if modified | Section 3b, Section 6, Section 10 |
| 3 | `LiveQuery Inner` | `core/src/livequery.rs:284` | Calls `node.unsubscribe_remote_predicate()` | Section 3c, Section 10 |
| 4 | `ReactorSubInner<E, Ev>` | `core/src/reactor/subscription.rs:41` | Calls `reactor.unsubscribe()` | Section 3c, Section 10 |
| 5 | `NodeInner<SE, PA>` | `core/src/node.rs:973` | Logging only (`notice_info!`) | Section 10 (explicitly listed as "No cleanup needed") |
| 6 | `ListenerGuard<T>` | `signals/src/broadcast.rs:137` | Removes listener from broadcast's listener map | Section 10 |
| 7 | `IVec<T, N>` | `core/src/util/ivec.rs:119` | Internal utility Drop (deallocates inline storage) | N/A — internal data structure, not relevant to TS port |

**Assessment**: All 6 meaningful `impl Drop` types are covered. `IVec` is an internal memory optimization utility and does not need coverage.

---

## 2. Complete Inventory of `Arc<T>` Usage

| # | Type / Field | File:Line | Purpose | Spec coverage |
|---|-------------|-----------|---------|---------------|
| 1 | `Entity(Arc<EntityInner>)` | `entity.rs:19` | Shared entity reference | Section 2 mapping table (Arc -> plain ref) |
| 2 | `EntityLiveQuery(Arc<Inner>)` | `livequery.rs:35` | Shared inner with Drop | Section 10 vicarious RAII table |
| 3 | `ReactorSubscription(Arc<ReactorSubInner>)` | `reactor/subscription.rs:52` | Shared inner with Drop | Section 10 vicarious RAII table |
| 4 | `Reactor(Arc<ReactorInner>)` | `reactor.rs:67` | Shared reactor state | Section 10 ("Node does NOT need Disposable") |
| 5 | `Node(Arc<NodeInner>)` | `node.rs:100` | Shared node state | Section 10 ("Node does NOT need Disposable") |
| 6 | `EntityResultSet(Arc<Inner>)` | `resultset.rs:46` | Shared result set | Section 10 ("EntityResultSet does NOT need Disposable") |
| 7 | `Broadcast<T>(Arc<Inner<T>>)` | `broadcast.rs:36` | Shared broadcast state | Covered implicitly (no Drop on Broadcast itself) |
| 8 | `Calculated<T>(Arc<Inner<T>>)` | `signals/calculated.rs:55` | Shared computed signal | Section 10 vicarious RAII table |
| 9 | `CallbackObserver(Arc<Inner>)` | `signals/observer/callback_observer.rs:9` | Shared observer | Section 10 vicarious RAII table |
| 10 | `SystemManager(Arc<Inner>)` | `system.rs:28` | Shared system state | Not explicitly listed (see Gap 1) |
| 11 | `CollectionSet(Arc<Inner>)` | `collectionset.rs:14` | Shared collection set | Not explicitly listed (infrastructure, no Drop) |
| 12 | `Context(Arc<dyn TContext>)` | `context.rs:24` | Trait object for context | Not listed (no Drop on context) |
| 13 | `LocalRetriever(Arc<LocalRetrieverInner>)` | `retrieval.rs:77` | Shared retriever state | Not listed (no Drop, infrastructure) |
| 14 | `Transaction.alive: Arc<AtomicBool>` | `transaction.rs:29` | Shared liveness flag | Section 14 |
| 15 | `Transaction.dyncontext: Arc<dyn TContext>` | `transaction.rs:26` | Shared context | Section 14 |
| 16 | `ReactObserver(Arc<Inner>)` (WASM) | `signals/react.rs:40` | Shared React observer | Not explicitly listed (see Gap 2) |
| 17 | `ReactObserver(Arc<Inner>)` (RN) | `signals/react_native.rs:78` | Shared RN observer | Not explicitly listed (see Gap 2) |
| 18 | `ValueCell<T>(Arc<RwLock<T>>)` | `signals/value.rs:5` | Shared mutable value | Covered by Arc->plain ref mapping |
| 19 | `Memo._subscription: ListenerGuard` | `signals/signal/memo.rs:18` | Owns Drop-carrying guard | Section 10 vicarious RAII table |
| 20 | `ReactorInner.watcher_set: Arc<Mutex<WatcherSet>>` | `reactor.rs:72` | Shared watcher set | Section 13 (PromiseMutex coverage) |
| 21 | `SubscriptionRelay(Arc<SubscriptionRelayInner>)` | `peer_subscription/client_relay.rs:91` | Shared relay state | Not listed (infrastructure, no Drop) |

**Assessment**: All Arc usages that carry Drop-implementing types are covered. The remaining unlisted Arc usages are infrastructure types with no `impl Drop` and no lifecycle management concerns.

---

## 3. Complete Inventory of `Weak<T>` Usage

| # | Type / Field | File:Line | Purpose | Spec Section 7 table |
|---|-------------|-----------|---------|---------------------|
| 1 | `WeakEntityLiveQuery(Weak<Inner>)` | `livequery.rs:58` | Break circular deps | COVERED |
| 2 | `WeakEntity(Weak<EntityInner>)` | `entity.rs:82` | Weak entity ref in `WeakEntitySet` | COVERED |
| 3 | `WeakNode(Weak<NodeInner>)` | `node.rs:108` | Gap fetcher holds weak ref | COVERED |
| 4 | `ListenerGuard.inner: Weak<Inner<T>>` | `signals/broadcast.rs:59` | Guard holds weak back-ref to Broadcast | COVERED |
| 5 | `WeakCallbackObserver(Weak<Inner>)` | `signals/observer/callback_observer.rs:21` | Prevent retain cycle observer<->signal | COVERED |
| 6 | `ReactObserverWeak(Weak<Inner>)` (WASM) | `signals/react.rs:42` | Prevent retain cycle in React observer listeners | NOT LISTED (see Gap 2) |
| 7 | `ReactObserverWeak(Weak<Inner>)` (RN) | `signals/react_native.rs:42` | Prevent retain cycle in RN observer listeners | NOT LISTED (see Gap 2) |
| 8 | `PNCounter.backend: Weak<PNBackend>` | `property/value/pn_counter.rs:20` | Weak ref to backend, upgraded on access | NOT LISTED (see Note 1) |
| 9 | `QueryGapFetcher.weak_node: Weak<NodeInner>` | `reactor/fetch_gap.rs:43` | Gap fetcher holds weak ref to avoid preventing Node cleanup | COVERED (same as WeakNode pattern) |

**Assessment**: 5 of 5 explicitly listed Weak patterns are correct. Two additional Weak patterns exist (ReactObserver variants, PNCounter.backend) — see gaps section.

---

## 4. Complete Inventory of `Mutex<T>` / `tokio::sync::Mutex` Usage

| # | Type / Field | File:Line | Mutex type | Spec coverage |
|---|-------------|-----------|------------|---------------|
| 1 | `ReactorInner.notify_lock` | `reactor.rs:74` | `tokio::sync::Mutex<()>` | Section 13 (PromiseMutex, already implemented) |
| 2 | `ReactorInner.subscriptions` | `reactor.rs:70` | `std::sync::Mutex` | Section 13 (can be eliminated — sync only) |
| 3 | `ReactorInner.watcher_set` | `reactor.rs:72` | `Arc<std::sync::Mutex<WatcherSet>>` | Section 13 (MISSING PromiseMutex flagged) |
| 4 | `Inner.state` (subscription_state) | `reactor/subscription_state.rs:90` | `std::sync::Mutex` | Section 13 context |
| 5 | `EntityResultSet Inner.state` | `resultset.rs:64` | `std::sync::Mutex<State>` | Section 6 (RefCell pattern) |
| 6 | `NodeInner.entity_subscription_state` | `node.rs:152` | `std::sync::Mutex` | Not explicitly listed (sync-only, can eliminate) |
| 7 | `SubscriptionRelayInner.subscriptions` | `client_relay.rs:53` | `std::sync::Mutex` | Not explicitly listed (infrastructure) |
| 8 | `LocalRetrieverInner.staged_events` | `retrieval.rs:81` | `std::sync::Mutex` | Not listed (sync-only, can eliminate) |
| 9 | `LWWBackend.field_broadcasts` | `property/backend/lww.rs:29` | `Mutex` | Not listed (sync-only, can eliminate) |
| 10 | `YrsBackend.previous_state` | `property/backend/yrs.rs:23` | `Mutex` | Not listed (sync-only, can eliminate) |
| 11 | `YrsBackend.field_broadcasts` | `property/backend/yrs.rs:24` | `Mutex` | Not listed (sync-only, can eliminate) |
| 12 | `ReactObserver.trigger_render` (WASM) | `signals/react.rs:50` | `Arc<std::sync::Mutex<Option<...>>>` | Not listed (WASM-specific, see Gap 2) |
| 13 | `ReactObserver.trigger_render` (RN) | `signals/react_native.rs:38` | `Mutex<Option<Arc<dyn StoreChangeCallback>>>` | Not listed (RN-specific, see Gap 2) |
| 14 | `ReactiveGraphObserver.bridges` | `signals/reactive_graph.rs:60` | `Mutex<HashMap<...>>` | Not listed (Leptos integration, see Note 2) |

**Assessment**: The critical `tokio::sync::Mutex` (notify_lock) is covered. The spec correctly identifies the MISSING watcher_set PromiseMutex. The remaining `std::sync::Mutex` instances are all sync-only access patterns that can be safely eliminated in TS (plain fields). The spec's Section 13 rule for when `std::sync::Mutex` can be eliminated correctly applies to all of them.

---

## 5. Complete Inventory of `RwLock<T>` Usage

| # | Type / Field | File:Line | Spec coverage |
|---|-------------|-----------|---------------|
| 1 | `EntityInner.state` | `entity.rs:53` | Eliminable (sync-only) |
| 2 | `WeakEntitySet` | `entity.rs:458` | Eliminable (sync-only) |
| 3 | `CollectionSet.collections` | `collectionset.rs:22` | Eliminable (sync-only) |
| 4 | `SystemManager inner fields` | `system.rs:35-42` | Section 13 (MISSING PromiseMutex flagged) |
| 5 | `Transaction.created_entity_ids` | `transaction.rs:33` | Eliminable (sync-only) |
| 6 | `SafeSet / SafeMap` | `util/safeset.rs:6`, `util/safemap.rs:7` | Eliminable (sync-only) |
| 7 | `LWWBackend.values` | `property/backend/lww.rs:28` | Eliminable (sync-only) |
| 8 | `PNBackend.values` | `property/backend/pn_counter.rs:25` | Eliminable (sync-only) |
| 9 | `Calculated.entries` | `signals/calculated.rs:26` | Eliminable (sync-only) |
| 10 | `CallbackObserver.entries` | `signals/observer/callback_observer.rs:19` | Eliminable (sync-only) |
| 11 | `Broadcast.listeners` | `signals/broadcast.rs:39` | Eliminable (sync-only) |
| 12 | `ReactObserver.entries` (WASM) | `signals/react.rs:47` | Eliminable (sync-only) |
| 13 | `ReactObserver.entries` (RN) | `signals/react_native.rs:33` | Eliminable (sync-only) |
| 14 | `Memo.cached` | `signals/memo.rs:16` | Eliminable (sync-only) |
| 15 | `ValueCell / ReadValueCell` | `signals/value.rs:5,8` | Eliminable (sync-only) |
| 16 | `OBSERVER_STACK` (multithread) | `signals/context.rs:64` | Section 12 (Observer Stack Context) |

**Assessment**: All `RwLock` usages are covered either explicitly or by the Section 13 elimination rule. The SystemManager RwLock fields are correctly flagged as needing PromiseMutex serialization.

---

## 6. Complete Inventory of `RefCell<T>` Usage (Rust std::cell::RefCell)

| # | Type / Field | File:Line | Purpose | Spec coverage |
|---|-------------|-----------|---------|---------------|
| 1 | `OBSERVER_STACK` (singlethread) | `signals/context.rs:15` | Thread-local observer stack | Section 12 (Observer Stack Context) |

**Assessment**: Only one actual `RefCell` usage exists in the codebase (the thread-local observer stack). The spec's Section 6 "RefCell Pattern" is about a *new TS class* that maps Rust's `Mutex`/`RwLock` guard-on-drop pattern (specifically `ResultSetWrite`), not a 1:1 port of Rust's `RefCell`. This is correct and well-explained.

---

## 7. Complete Inventory of Lifetime Parameters

| # | Lifetime | Type | File:Line | Purpose | Spec coverage |
|---|----------|------|-----------|---------|---------------|
| 1 | `'a` | `ResultSetWrite<'a, E>` | `resultset.rs:90` | Guard borrows EntityResultSet | Section 6, Section 14 |
| 2 | `'a` | `ResultSetRead<'a, E>` | `resultset.rs:98` | Read guard borrows EntityResultSet | Section 6 (covered by scoped pattern) |
| 3 | `'a` | `Ref<'a, T>` | `signals/broadcast.rs:50` | Listen-only broadcast reference | Not listed (ephemeral borrow, no TS equivalent needed) |
| 4 | `'a` | `CandidateChangeIterator<'a, C>` | `reactor/candidate_changes.rs:15` | Iterator borrow | Not listed (ephemeral borrow, iterator pattern) |
| 5 | `'static` | Various trait bounds | Multiple | Owned closures, thread safety | Section 14 (alive flag enforcement) |

**Assessment**: The two significant lifetime patterns (`ResultSetWrite`, `MutableBorrow`) are fully covered in Sections 6 and 14. The remaining lifetime parameters are ephemeral borrows that naturally map to synchronous function scopes in TS and require no special handling.

---

## 8. Vicarious RAII Coverage

| # | Type | Owns Drop-carrying field(s) | Spec Section 10 | Status |
|---|------|---------------------------|-----------------|--------|
| 1 | `EntityLiveQuery` | `Arc<Inner>` where `Inner: Drop`; also owns `ReactorSubscription` | Listed | COVERED |
| 2 | `LiveQuery<R>` | `EntityLiveQuery` (transitive) | Listed | COVERED |
| 3 | `ReactorSubscription` | `Arc<ReactorSubInner>` where `ReactorSubInner: Drop` | Listed | COVERED |
| 4 | `SubscriptionGuard` | `Box<dyn Any>` containing `ListenerGuard: Drop` | Listed | COVERED |
| 5 | `Calculated<T>` | `RwLock<HashMap<..., SubscriptionEntry>>` where each entry has `ListenerGuard: Drop` | Listed | COVERED |
| 6 | `CallbackObserver` | `RwLock<HashMap<..., SubscriptionEntry>>` where each entry has `ListenerGuard: Drop` | Listed | COVERED |
| 7 | `Memo` | `_subscription: ListenerGuard` | Listed | COVERED |
| 8 | `signal::ListenerGuard` (wrapper) | `Box<dyn TListenerGuard>` wrapping `broadcast::ListenerGuard<T>: Drop` | Not explicitly listed | MINOR GAP (see below) |
| 9 | `ReactObserver` (WASM) | `HashMap<..., ListenerEntry>` where each entry has `ListenerGuard: Drop` | Not listed | GAP 2 |
| 10 | `ReactObserver` (RN) | `HashMap<..., ListenerEntry>` where each entry has `ListenerGuard: Drop` | Not listed | GAP 2 |
| 11 | `BridgeSource` | `_guard: ListenerGuard` | Not listed | Note 2 |

**Assessment**: All 7 explicitly listed vicarious RAII types are correct and complete. The `signal::ListenerGuard` wrapper is itself a vicarious RAII type wrapping `broadcast::ListenerGuard<T>`, but since it is used as a field in all the listed types, the cascade is implicitly covered. The ReactObserver variants are a minor gap.

---

## 9. Gaps Found

### Gap 1: ReactObserver (WASM and React Native) — Vicarious RAII Not Listed

**Severity**: Low (TS port of ReactObserver likely follows a different pattern — managed by React hooks)

**Details**: Both `ReactObserver` in `signals/react.rs` and `signals/react_native.rs` own `HashMap<BroadcastId, ListenerEntry>` where each `ListenerEntry` contains a `ListenerGuard` (which has `impl Drop`). These are vicarious RAII types. When a `ReactObserver` is dropped, all its listener guards drop, unsubscribing from broadcasts.

**Why this is low severity**: In the TS port, the React observer pattern will likely be implemented entirely in JS/TS using the React hooks pattern (`useEffect` cleanup). The Rust `ReactObserver` exists for WASM and UniFFI bindings that won't be needed in a pure-TS implementation. The spec's Section 12 already covers the observer stack pattern and the `useEffect` cleanup pattern for React.

**Suggestion**: Add a note to Section 10's "Types that do NOT need Disposable" table:

> | `ReactObserver` (WASM/RN) | TS port uses native React hooks (`useEffect` cleanup) instead of porting the Rust observer. The Rust type exists for WASM/UniFFI bindings. |

### Gap 2: `PNCounter.backend: Weak<PNBackend>` Not Listed in WeakRef Table

**Severity**: Very low (property values are ephemeral, not lifecycle-critical)

**Details**: `PNCounter` (and by analogy `LWW`, `YrsString`) hold a `Weak<Backend>` reference that is upgraded on every access. This is not listed in Section 7's Weak usage table.

**Why this is very low severity**: In the TS port, property value types will hold direct references to their backends (since the backend's lifetime is tied to the Entity, which is GC-managed). The Weak pattern in Rust exists because the backend is `Arc`-wrapped and the property value is a non-owning view. In TS, this naturally becomes a plain reference — no `WeakRef` needed.

**Suggestion**: Add a note to Section 7 or Section 2:

> **Property value types** (`LWW`, `YrsString`, `PNCounter`) hold `Weak<Backend>` in Rust to avoid extending the backend's lifetime beyond the entity. In TS, these hold plain references — the Entity (and its backends) are GC-managed, and property values are always used within a live Entity context.

---

## 10. Notes (Not Gaps)

### Note 1: `signal::ListenerGuard` Wrapper

The `signal::ListenerGuard` struct (`signals/src/signal.rs:16`) is a type-erased wrapper around `broadcast::ListenerGuard<T>`. It is itself vicarious RAII (owns a `Box<dyn TListenerGuard>` which contains a `broadcast::ListenerGuard<T>: Drop`). The spec does not list it separately, but this is correct — it is always used *as a field* in the listed vicarious RAII types (Calculated, CallbackObserver, Memo, SubscriptionGuard). The cascade is implicit and the spec's guidance ("each owner's `onDispose()` must call `dispose()` on owned Disposable fields") covers it.

### Note 2: ReactiveGraphObserver / BridgeSource

`ReactiveGraphObserver` (`signals/reactive_graph.rs`) owns `BridgeSource` instances that each contain a `ListenerGuard`. This is a Leptos/reactive_graph integration point. It is not listed in the spec. This is acceptable because:
- The TS port is not targeting Leptos
- If a reactive_graph integration is needed in TS, it would follow the same observer pattern already documented in Section 12

### Note 3: Observer Stack Context Pattern — Comprehensive

Section 12's "Observer Stack Context (Signals)" correctly identifies the `RefCell<Vec<Arc<dyn Observer>>>` / `RwLock<Vec<...>>` pattern in `signals/context.rs`. The try/finally guidance for push/pop is accurate and critical. This pattern maps to a module-level array in TS, and the spec correctly notes the try/finally requirement.

### Note 4: `IVec<T, N>` internal Drop

`core/src/util/ivec.rs:119` has `impl Drop for IVec<T, N>` — this is an internal memory optimization (inline small-vector). It is not relevant to the TS port and correctly omitted from the spec.

---

## 11. Cross-Reference Summary

| Spec Section | Patterns Covered | Complete? |
|-------------|-----------------|-----------|
| Section 2 (Core Mapping Table) | Drop, Arc, Weak, Mutex, RwLock, RefCell, AtomicBool, Lifetime | YES |
| Section 3 (Classification Table) | All 10 TS types classified | YES |
| Section 6 (RefCell Pattern) | ResultSetWrite guard-on-drop -> withMut | YES |
| Section 7 (WeakRef Table) | 5 of 7 Weak usages (missing ReactObserver variants, PNCounter) | MOSTLY (see Gaps) |
| Section 10 (impl Drop table) | 6 of 6 meaningful Drop impls | YES |
| Section 10 (Vicarious RAII table) | 7 of 9 vicarious RAII types (missing ReactObserver variants) | MOSTLY (see Gap 1) |
| Section 10 (NOT Disposable table) | Node, EntityResultSet, Entity | YES |
| Section 12 (Observer Stack) | try/finally for push/pop | YES |
| Section 13 (PromiseMutex) | tokio::sync::Mutex -> PromiseMutex; MISSING items flagged | YES |
| Section 14 (Lifetime Enforcement) | MutableBorrow, Transaction alive checks | YES |

---

## 12. Final Assessment

The spec is thorough and well-structured. It correctly identifies all critical patterns and provides actionable mapping rules. The two gaps identified are both low-severity and relate to platform-specific integration types (ReactObserver for WASM/UniFFI, PNCounter backend Weak) that will either not be ported 1:1 or will naturally resolve to simpler TS patterns.

**Verdict: PASS WITH NOTES**

The spec is ready for use as the authoritative reference for the TS port's lifecycle management. The suggested additions in Gaps 1 and 2 are quality improvements, not blockers.
