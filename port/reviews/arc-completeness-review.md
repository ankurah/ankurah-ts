# Arc/Weak/Borrow Completeness Review

Reviewer: completeness-reviewer
Date: 2026-03-14

---

## Methodology

Scanned all `.rs` files in `ankurah/core/src/` and `ankurah/signals/src/` for:
- Every `Arc<T>` usage
- Every `Weak<T>` usage
- Every `&T`/`&mut T` stored in struct fields
- Every `impl Drop` that interacts with Arc

Cross-referenced each against the TS port in `ankurah-ts/packages/` to determine current handling.

---

## Classification Legend

- **True shared ownership** — Multiple independent owners; last drop triggers cleanup.
- **Single-owner clonability** — Arc used for cheap cloning or interior mutability, but effectively one logical owner.
- **Type-erasure wrapper** — Arc wrapping `dyn Trait` for heap allocation + shared access; no multi-owner Drop semantics.
- **Weak cycle-breaker** — Weak used to prevent reference cycles where the parent Arc outlives the child.

---

## 1. Arc<T> Usage Catalog

### 1.1 core/src/entity.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `Entity(Arc<EntityInner>)` | Entity is Clone; multiple holders (resultsets, queries, transaction forks, WeakEntitySet) | **True shared ownership** | Plain class, GC handles it | **No** — `EntityInner` has no `impl Drop`. JS GC is equivalent to "last Arc drops." No cleanup to trigger. |
| `TemporaryEntity(Arc<EntityInner>)` | Same pattern, ephemeral | Single-owner clonability | Plain class | No |
| `WeakEntitySet(Arc<RwLock<BTreeMap<...>>>)` | Shared container across node/context/system | Single-owner clonability | Plain class | No |
| `Arc<dyn PropertyBackend>` in `EntityInnerState.backends` | Type-erased backend shared between entity and forks | Type-erasure wrapper | Plain `PropertyBackend` references | No — no Drop on PropertyBackend |
| `Arc<AtomicBool>` in `EntityKind::Transacted` | Transaction liveness flag shared between trx and entity snapshot | **True shared ownership** of flag | `{ value: boolean }` shared reference | **No** — correctly handled. Shared mutable boolean. No Drop. |

**Verdict**: Entity's Arc provides cheap Clone + shared references. In JS, object references already do this. No Drop on `EntityInner` means no cleanup semantics to preserve. **Current TS handling is correct.**

### 1.2 core/src/reactor.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `Reactor(Arc<ReactorInner>)` | Reactor is Clone; shared between Node, LiveQuery, ReactorSubscription | Single-owner clonability | Plain class | No — no Drop on ReactorInner |
| `Arc<Mutex<WatcherSet>>` | Shared between Reactor and all Subscriptions | Shared mutable state | Direct field, no mutex needed | No |

### 1.3 core/src/reactor/subscription.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `ReactorSubscription(Arc<ReactorSubInner>)` | **Key case.** Multiple clones of subscription handle. `ReactorSubInner` has `impl Drop` that calls `reactor.unsubscribe()`. | **True shared ownership with Drop** | `ReactorSubInner` has explicit `drop()` method; TS uses callback-based unsubscribe | **Potentially yes** — but see analysis below |

**Analysis of ReactorSubscription**: In Rust, `ReactorSubInner` has `impl Drop` which calls `reactor.unsubscribe(subscription_id)`. The `ReactorSubscription` is cloned (e.g., `subscription.clone()` in `evaluate_changes`), and unsubscribe only fires when the **last** clone drops.

In the TS port, `ReactorSubInner` is created once and its `drop()` is called explicitly. The question is: **can multiple independent owners hold ReactorSubscription clones in TS?**

Looking at the Rust code:
- `ReactorSubscription` is stored in `LiveQuery.Inner.subscription` (one owner)
- It's cloned in `reactor.subscribe()` to return to callers
- `Clone` on line 95-97 clones the Arc

In TS, `ReactorSubscription` is currently not using Arc. The subscription is stored in `LiveQuery` and the `ReactorSubInner.drop()` is called when the LiveQuery is cleaned up. This is correct **only if there's a single owner** — which appears to be the case in practice (LiveQuery is the sole owner of its subscription).

**Verdict**: The Rust code technically allows shared ownership of `ReactorSubscription`, but the TS port correctly identifies that there is effectively a single owner (LiveQuery). If a future Rust change shares ReactorSubscription across multiple owners, Arc<T> would be needed. **Current TS handling is acceptable but fragile.**

### 1.4 core/src/reactor/subscription_state.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `Subscription(Arc<Inner>)` | Internal subscription state, cloned within reactor | Single-owner clonability | Plain class | No — no Drop |
| `Arc<Mutex<WatcherSet>>` | Shared with Reactor | Shared ref | Direct reference | No |
| `Arc<dyn GapFetcher<E>>` | Type-erased gap fetcher | Type-erasure wrapper | Interface reference | No |

### 1.5 core/src/livequery.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `EntityLiveQuery(Arc<Inner>)` | LiveQuery is Clone; `Inner` has `impl Drop` that calls `node.unsubscribe_remote_predicate()` | **True shared ownership with Drop** | Plain class with explicit cleanup | **Potentially yes** — see analysis |

**Analysis of EntityLiveQuery**: `Inner` has `impl Drop` that calls `self.node.unsubscribe_remote_predicate(self.query_id)`. The LiveQuery is cloned for:
- Spawning initialization tasks (`me2 = me.clone()`)
- Creating weak references (`me.weak()`)
- Self-referential async calls

The clones are all **temporary** — they exist only during async task execution and are dropped when the task completes. The "real" owner is the caller who created the LiveQuery. Drop fires when the last temporary clone is also done.

In TS, the async tasks capture `this` via closures, which keeps the object alive via GC. The cleanup (`unsubscribe_remote_predicate`) is called explicitly.

**Verdict**: Clones are temporary/async. JS closures naturally keep the object alive during async work. The only risk is if `drop()`/cleanup is called while an async task still references the object — but `assertNotDropped()` guards would catch this. **Current TS handling is correct.**

### 1.6 core/src/node.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `Node(Arc<NodeInner>)` | Node is Clone; extensively shared | Single-owner clonability | Plain class | No — no Drop on NodeInner |
| `Arc<PeerState>` in `peer_connections` | Shared peer state | Single-owner clonability | Plain class | No |

### 1.7 core/src/context.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `Context(Arc<dyn TContext>)` | Type-erased context | Type-erasure wrapper | Interface-based | No |
| `Arc<dyn TContext>` in Transaction | Shared between Transaction and Context | Type-erasure + shared ref | Interface reference | No |

### 1.8 core/src/transaction.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `Arc<AtomicBool>` for `alive` flag | Shared between Transaction and entity snapshots | **True shared ownership** of flag | `{ value: boolean }` shared reference | **No** — correctly handled |

**`impl Drop for Transaction`**: Sets `alive` to false. TS equivalent explicitly sets `trxAlive.value = false`.

### 1.9 core/src/collectionset.rs, system.rs, resultset.rs, retrieval.rs, storage.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `CollectionSet(Arc<Inner>)` | Clone wrapper | Single-owner clonability | Plain class | No |
| `SystemManager(Arc<Inner>)` | Clone wrapper | Single-owner clonability | Plain class | No |
| `EntityResultSet(Arc<Inner>)` | Clone wrapper, shared between subscription_state, LiveQuery, gap_fetcher | Single-owner clonability | Plain class | No |
| `LocalRetriever(Arc<LocalRetrieverInner>)` | Clone wrapper | Single-owner clonability | Plain class | No |
| `StorageCollectionWrapper(Arc<dyn StorageCollection>)` | Type erasure | Type-erasure wrapper | Interface | No |
| `Arc<SE>` (storage engine) | Shared engine ref | Single-owner clonability | Plain class | No |

### 1.10 core/src/property/value/*.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `LWW.backend: Arc<LWWBackend>` | Shared between entity and property view | Shared ref | Plain reference | No |
| `YrsString.backend: Arc<YrsBackend>` | Same | Shared ref | Plain reference | No |
| `PNCounter.backend: Weak<PNBackend>` | **Weak** — non-owning ref to backend | Weak cycle-breaker | See Weak section below | |

### 1.11 core/src/reactor/fetch_gap.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `QueryGapFetcher.weak_node: Weak<NodeInner>` | Prevents GapFetcher from keeping Node alive | **Weak cycle-breaker** | See Weak section | |

### 1.12 core/src/peer_subscription/client_relay.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `SubscriptionRelay.inner: Arc<SubscriptionRelayInner>` | Clone wrapper | Single-owner clonability | Not yet ported | N/A |
| `Arc<Content<CD>>` | Shared content | Shared ref | Not yet ported | N/A |
| `Arc<dyn TNode<CD>>` | Type-erased node ref | Type-erasure | Not yet ported | N/A |

### 1.13 signals/src/broadcast.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `Broadcast(Arc<Inner>)` | Broadcast is Clone; shared between sender and `Ref` views | Single-owner clonability | Plain class | No — `ListenerGuard` uses Weak, not Arc |
| `BroadcastListener::Payload(Arc<dyn Fn(T)>)` | Type-erased callback | Type-erasure | Plain function reference | No |
| `ListenerGuard.inner: Weak<Inner>` | **Weak** — guard doesn't keep broadcast alive | **Weak cycle-breaker** | See Weak section | |

### 1.14 signals/src/react_native.rs, react.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `ReactObserver(Arc<Inner>)` | Clone wrapper; no Drop on Inner | Single-owner clonability | Not directly ported (React integration is TS-native) | N/A |

### 1.15 signals/src/signal/calculated.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `Calculated(Arc<Inner>)` | Clone wrapper + `Observer for Arc<Inner>` uses Arc to implement the observer pattern | Single-owner clonability | Plain class | No |

### 1.16 signals/src/observer/callback_observer.rs

| Rust Type | Usage | Category | TS Handling | Arc Needed in TS? |
|-----------|-------|----------|-------------|-------------------|
| `CallbackObserver(Arc<Inner>)` | Clone wrapper + weak refs for listeners | Single-owner clonability | Plain class | No |

---

## 2. Weak<T> Usage Catalog

### 2.1 core/src/entity.rs — `WeakEntity(Weak<EntityInner>)`

**Purpose**: `WeakEntitySet` stores `WeakEntity` references so the set doesn't prevent entity GC. Entities are held strongly by resultsets/queries; the set just provides lookup.

**TS handling**: `WeakEntitySet` uses JS `WeakRef<Entity>` and `FinalizationRegistry`. **Correct.**

### 2.2 core/src/reactor/fetch_gap.rs — `QueryGapFetcher.weak_node: Weak<NodeInner>`

**Purpose**: GapFetcher is stored in `QueryState` within subscriptions. If it held a strong ref to Node, it would create a cycle: Node -> Reactor -> Subscription -> QueryState -> GapFetcher -> Node.

**TS handling**: Currently uses a plain reference to Node. This creates a reference cycle, but JS GC handles cycles fine. The risk is that GapFetcher keeps Node alive when it shouldn't — but in practice, GapFetcher's lifetime is bounded by the subscription's lifetime, which is bounded by the Node's lifetime. **No bug, but semantically divergent.**

**Should TS use Weak?** Not for correctness. The Weak in Rust is about preventing the Arc cycle from leaking memory, not about cleanup semantics. JS GC handles cycles. **No action needed.**

### 2.3 core/src/livequery.rs — `WeakEntityLiveQuery(Weak<Inner>)`

**Purpose**: Used in `RemoteQuerySubscriber` impl to break a cycle: LiveQuery -> subscription -> relay -> LiveQuery. The weak ref allows the relay callback to check if the LiveQuery still exists.

**TS handling**: `WeakEntityLiveQuery` class wraps the LiveQuery reference. Checking the TS port... The TS `WeakEntityLiveQuery` does not use JS `WeakRef` — it holds a direct reference. However, the semantic is similar because JS GC handles cycles.

**Should TS use WeakRef?** It would be more faithful to the Rust semantics (upgrade() returning null if dropped), but since there's no real risk of accessing a dropped LiveQuery through this path in practice, it's acceptable. If the LiveQuery gains explicit `drop()` semantics, then WeakRef would prevent use-after-drop.

### 2.4 core/src/node.rs — `WeakNode(Weak<NodeInner>)`

**Purpose**: Used for passing a non-owning reference to Node where the caller shouldn't prevent Node cleanup.

**TS handling**: Not yet ported in a context that needs it.

### 2.5 core/src/property/value/pn_counter.rs — `PNCounter.backend: Weak<PNBackend>`

**Purpose**: PNCounter holds a Weak to its backend. The Entity (via `EntityInnerState.backends`) owns the Arc. If the entity is dropped, the backend goes away.

**TS handling**: Not directly observable in port yet. PNCounter is not heavily used.

**Should TS use Weak?** No. The PNCounter's lifetime is always bounded by the Entity that created it. If the Entity is GC'd, the PNCounter becomes unreachable too.

### 2.6 signals/src/broadcast.rs — `ListenerGuard.inner: Weak<Inner>`

**Purpose**: The listener guard holds a Weak to the broadcast inner. This prevents a guard from keeping the broadcast alive. When the broadcast is dropped, the guard's drop() silently does nothing (can't upgrade).

**TS handling**: `ListenerGuard` stores a nullable reference to `Inner` and sets it to null in `drop()`. The Broadcast's `Inner` is directly referenced, not via WeakRef.

**Should TS use WeakRef?** Not necessary. The `drop()` method on ListenerGuard explicitly removes the listener, and if the Broadcast is GC'd first, the ListenerGuard's drop is a no-op. The current nullable pattern achieves the same effect.

### 2.7 signals/src/ — `ReactObserverWeak`, `WeakCallbackObserver`

**Purpose**: Signal listeners create weak references to observers so that signals don't prevent observer GC.

**TS handling**: The TS signal implementation uses direct references inside closures, with the closures held in ListenerGuards. When the guard is dropped, the closure is removed.

**Should TS use WeakRef?** Not necessary. The closure/guard lifecycle management achieves the same effect.

---

## 3. Borrowed References in Struct Fields (&T / &mut T)

### 3.1 core/src/reactor/candidate_changes.rs

```rust
struct CandidateQueryIter<'a, C> {
    changes: &'a Arc<Vec<C>>,  // borrowed from CandidateChanges
}
```

**Category**: Lifetime-bounded borrow — lives only during iteration.

**TS handling**: Direct reference in iteration. **Correct** — no Borrow<T> needed for temporary iteration references.

### 3.2 core/src/retrieval.rs

```rust
pub struct EphemeralNodeRetriever<'a, SE, PA, C> {
    pub node: &'a Node<SE, PA>,
    pub cdata: &'a C,
}
```

**Category**: Lifetime-bounded borrow — retriever lives only for the duration of a single operation.

**TS handling**: Direct references. **Correct** — the retriever is ephemeral.

### 3.3 signals/src/broadcast.rs

```rust
pub struct Ref<'a, T>(&'a Broadcast<T>);
```

**Category**: Lifetime-bounded borrow — read-only reference to broadcast.

**TS handling**: The `BroadcastRef` class holds a direct reference. **Correct.**

**General verdict on Borrow<T>**: None of the Rust struct fields use `&T` in long-lived structs where ownership ambiguity could cause bugs in TS. All borrowed references are lifetime-bounded (function-scoped or iterator-scoped). **Borrow<T> and BorrowMut<T> are not needed for any current struct fields** — they would only be needed if a TS struct held a non-owning reference to something with Drop semantics where the lint needs to distinguish "must drop" from "must not drop" fields.

---

## 4. impl Drop Interacting with Arc

### 4.1 `impl Drop for ReactorSubInner` (core/src/reactor/subscription.rs:41)

```rust
impl<E, Ev> Drop for ReactorSubInner<E, Ev> {
    fn drop(&mut self) {
        let _ = self.reactor.unsubscribe(self.subscription_id);
    }
}
```

**Arc involvement**: `ReactorSubscription` wraps `Arc<ReactorSubInner>`. Drop fires when last Arc clone drops.

**TS handling**: `ReactorSubInner.drop()` calls unsubscribe callback. Since there's effectively a single owner (LiveQuery), this is correct.

**Risk**: If `ReactorSubscription` is ever cloned and the clone outlives the original, the TS version would call `drop()` too early (on the original) while the clone still needs the subscription. Arc<T> in TS would fix this. **Current risk: Low** — code review shows no independent clones that outlive the owner.

### 4.2 `impl Drop for Inner` (core/src/livequery.rs:284)

```rust
impl Drop for Inner {
    fn drop(&mut self) { self.node.unsubscribe_remote_predicate(self.query_id); }
}
```

**Arc involvement**: `EntityLiveQuery(Arc<Inner>)`. Drop fires when last reference drops.

**TS handling**: Explicit cleanup in LiveQuery. Async task clones are temporary.

**Risk**: Same as 4.1 — if async tasks hold the last reference and Drop fires after explicit cleanup, there's a double-unsubscribe. The `let _` in Rust silently ignores errors from double-unsubscribe. TS should do the same. **Low risk.**

### 4.3 `impl Drop for Transaction` (core/src/transaction.rs:126)

```rust
impl Drop for Transaction {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
}
```

**Arc involvement**: Transaction itself is not in Arc. `alive` field is `Arc<AtomicBool>` shared with entity snapshots.

**TS handling**: Transaction extends Drop, sets `trxAlive.value = false`. **Correct.**

### 4.4 `impl Drop for ListenerGuard` (signals/src/broadcast.rs:137)

```rust
impl<T> Drop for ListenerGuard<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.listeners.write().unwrap().remove(&self.id);
        }
    }
}
```

**Arc involvement**: ListenerGuard holds `Weak<Inner>`. Drop only acts if broadcast is still alive.

**TS handling**: `ListenerGuard extends Drop`. Drop removes listener from broadcast if reference is non-null. **Correct.**

---

## 5. Summary: Does Arc<T> Fix Real Bugs?

| Location | Would Arc<T> in TS fix a real bug? | Why/Why not |
|----------|-------------------------------------|-------------|
| Entity | No | No Drop on EntityInner. JS GC = last Arc drop. |
| ReactorSubscription | **Latent risk** | If ever shared between independent owners, early drop = double-unsubscribe or orphaned subscription. Currently single-owner. |
| EntityLiveQuery | **Latent risk** | Same pattern as ReactorSubscription. Currently single-owner with temporary async clones. |
| Transaction.alive | No | Shared boolean, no Drop semantics on the bool itself. |
| Reactor/Node/CollectionSet/etc | No | All single-owner clonability patterns without Drop. |
| Broadcast | No | ListenerGuard uses nullable ref correctly. |
| PropertyBackend | No | No Drop semantics. |
| Weak usages | No | JS GC handles cycles; WeakRef optional for correctness. |
| Borrow<T>/BorrowMut<T> | No | All borrows are lifetime-bounded; no long-lived struct fields need ownership disambiguation. |

---

## 6. Conclusions

### The spec is complete for current usage

The ownership spec correctly describes Arc<T>, Weak<T>, Borrow<T>, and BorrowMut<T>. The **actual codebase** does not currently require any of these types for correctness. Every case falls into one of:

1. **No Drop on inner type** — JS GC is equivalent to "last Arc drops"
2. **Single logical owner** — explicit `drop()` call is sufficient
3. **Lifetime-bounded borrows** — no ownership ambiguity in struct fields

### Two cases warrant monitoring

1. **ReactorSubscription** (`Arc<ReactorSubInner>` with `impl Drop`) — If the Rust code ever evolves to share `ReactorSubscription` across truly independent owners, the TS port must either use `Arc<T>` or add refcounting.

2. **EntityLiveQuery** (`Arc<Inner>` with `impl Drop`) — Same pattern. Currently safe because async clones are temporary, but if a clone were stored independently, cleanup would misfire.

### Arc<T> is ceremony for now, insurance for later

For the current Rust codebase, adding `Arc<T>` to the TS port would be pure ceremony — it would not fix any existing bug. However, having the provided type available means the port can adopt it if the Rust code evolves to genuinely share ownership of Drop-implementing types.

### Borrow<T>/BorrowMut<T> have zero current use cases in struct fields

All `&T`/`&mut T` references in Rust structs are lifetime-bounded (iterators, temporary retrievers, broadcast Refs). None are long-lived struct fields that would benefit from `Borrow<T>` disambiguation in TS. The types exist in the spec for future use if a Rust struct stores a non-owning reference alongside owned Drop fields.
