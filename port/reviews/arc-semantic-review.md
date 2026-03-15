# Arc/Weak/Borrow Semantic Review

**Reviewer**: semantic-reviewer
**Date**: 2026-03-14
**Spec reviewed**: `/port/ownership.md` (Arc<T>, Weak<T>, Borrow<T>, BorrowMut<T> additions)
**Rust source scanned**: `/ankurah/core/src/**/*.rs` (all Arc<, Weak<, impl Drop patterns)

---

## 1. Is Arc<T> actually needed?

### Finding: Yes, but only for 2 types. The rest are false positives.

I found exactly **2 types** in ankurah/core where `Arc<T>` wraps a type with `impl Drop`:

| Type | Arc wrapper | Drop impl | Multi-owner? | Cleanup action |
|------|-----------|-----------|-------------|----------------|
| `ReactorSubInner<E, Ev>` | `ReactorSubscription(Arc<ReactorSubInner>)` | Yes (`fn drop` calls `reactor.unsubscribe()`) | **Yes** -- `Clone for ReactorSubscription` clones the Arc (line 96). Multiple handles share one inner. Only the last drop unsubscribes. | Unsubscribes from reactor |
| `Inner` (livequery) | `EntityLiveQuery(Arc<Inner>)` | Yes (`fn drop` calls `node.unsubscribe_remote_predicate()`) | **Yes** -- `#[derive(Clone)]` on `EntityLiveQuery` clones the Arc. `WeakEntityLiveQuery` uses `Arc::downgrade`. Multiple owners exist (main handle + spawned tasks + remote subscriber). | Unsubscribes remote predicate |

All other Arc usage in the codebase falls into these categories:

- **Arc wrapping non-Drop types** (no cleanup semantics): `Entity(Arc<EntityInner>)`, `Node(Arc<NodeInner>)`, `Reactor(Arc<ReactorInner>)`, `EntityResultSet(Arc<Inner>)`, `SystemManager(Arc<Inner>)`, `CollectionSet(Arc<Inner>)`, `Context(Arc<dyn TContext>)`, `LocalRetriever(Arc<..>)`. None of these inner types implement Drop. They use Arc purely for shared access -- in TS these should be plain objects with reference semantics (JS objects are already reference-counted by the GC).

- **Arc wrapping data** (no ownership semantics): `Arc<Vec<C>>` in CandidateChanges, `Arc<Mutex<...>>` for shared state, `Arc<dyn PropertyBackend>`, `Arc<SE>` for storage engines. These are shared-data patterns, not ownership patterns. Plain TS references suffice.

- **Arc for trait objects**: `Arc<dyn TContext>`, `Arc<dyn GapFetcher>`, `Arc<dyn TNode>`, `Arc<dyn StorageCollection>`. These are interface-based polymorphism. In TS, a plain interface reference is sufficient.

### Verdict: Arc<T> is genuinely needed for exactly ReactorSubscription and EntityLiveQuery.

In both cases, multiple owners actually share the same inner, and only the last drop should trigger cleanup. This is the canonical Arc use case.

**However**, the spec should acknowledge that the vast majority of Rust's `Arc<T>` usage maps to plain `T` in TS. Only `Arc<T> where T: Drop` with actual multi-owner cloning requires the provided `Arc<T>` type. The spec's mapping table says `Arc<T>` -> `Arc<T>` unconditionally, which would cause massive over-application. A translator would wrap `Node`, `Entity`, `Reactor`, etc. in `Arc<T>` unnecessarily.

**Recommendation**: Add a qualification: "Arc<T> -> Arc<T> **only when T extends Drop and multiple owners clone the Arc**. Otherwise Arc<T> -> plain T (JS reference semantics are sufficient)."

---

## 2. Does the `.value` noise problem return?

### Finding: Partially addressed, but still a problem for ReactorSubscription.

The spec doesn't show how field access works through `Arc<T>`. Looking at the Rust code:

```rust
// Rust -- Arc<T> dereferences transparently via Deref
let sub: ReactorSubscription = ...; // wraps Arc<ReactorSubInner>
sub.0.subscription_id   // transparent access through Arc
sub.0.reactor.unsubscribe(...)
```

In TS, if `Arc<T>` stores the inner as a `.value` property:
```typescript
const sub = Arc.new(new ReactorSubInner(...));
sub.value.subscriptionId  // noise on every access
```

But looking at how `ReactorSubscription` actually works in Rust -- it's a newtype wrapper `struct ReactorSubscription(Arc<ReactorSubInner>)`. The public API never exposes the inner directly. Callers use methods like `sub.id()`, `sub.subscribe(...)`. They never reach through the Arc.

Same for `EntityLiveQuery(Arc<Inner>)` -- all access is through methods on `EntityLiveQuery`, never through the Arc directly.

**So the `.value` noise problem is mitigated by the newtype pattern**: the Arc is an implementation detail hidden inside the struct. The TS translation would be:

```typescript
class ReactorSubscription extends Disposable {
    private inner: Arc<ReactorSubInner>;
    // public methods delegate to this.inner.value.xxx
    // but callers never see .value
}
```

### Residual concern: internal code within the class still has `.value` noise.

Every method in `ReactorSubscription` and `EntityLiveQuery` would need `this.inner.value.subscriptionId` instead of `this.inner.subscriptionId`. This is ugly but contained within the class implementation.

**Alternative**: Since `ReactorSubscription` and `EntityLiveQuery` are the only consumers, they could embed the refcount directly (extend `Disposable` with a shared refcount) rather than wrapping a generic `Arc<T>`. This would eliminate `.value` entirely. The generic `Arc<T>` type would then have zero actual users.

**Verdict**: The `.value` problem is manageable but raises the question of whether a generic `Arc<T>` is worth building for only 2 use sites.

---

## 3. Borrow<T> / BorrowMut<T> -- needed or noise?

### Finding: These add type-system noise with zero runtime behavior. A lint annotation is superior.

The spec says `Borrow<T>` marks "I'm using this but don't own it" so the lint can distinguish owned fields from borrowed fields. Let's examine what this means concretely.

In Rust, `&T` is enforced by the compiler -- you literally cannot call `.drop()` on a `&T`. The borrow checker prevents it. In TS, `Borrow<T>` would be a type alias or wrapper. What does it actually do?

**Option A: Type alias** -- `type Borrow<T> = T`. Zero runtime cost, but TypeScript's structural typing means it's identical to `T`. The lint rule can't distinguish `Borrow<Foo>` from `Foo` at runtime. The lint would need to inspect type annotations, which ESLint can with type-aware rules, but this is fragile.

**Option B: Wrapper class** -- `class Borrow<T> { constructor(public readonly value: T) {} }`. Now we're back to `.value` noise on every access, and we need to unwrap at every call site. For something that has no runtime behavior, this is pure ceremony.

**Option C: Lint annotation** -- `/* @borrowed */ private ref: Foo`. The lint rule reads the annotation. Zero runtime overhead, zero access noise, explicitly communicates intent. This is what the spec should recommend.

The spec's example:
```typescript
class Foo extends Drop {
    private owned: Arc<Bar>;        // I own this -- must drop
    private borrowed: Borrow<Baz>;  // Someone else owns this -- don't drop
}
```

With a lint annotation instead:
```typescript
class Foo extends Drop {
    private owned: Arc<Bar>;        // I own this -- must drop
    /** @borrowed */ private ref: Baz;  // Someone else owns this -- don't drop
}
```

Same clarity, no wrapper type, no `.value` access penalty.

**Verdict**: `Borrow<T>` and `BorrowMut<T>` should be removed from the provided types. Use `/** @borrowed */` lint annotations instead. The mutability distinction (`Borrow` vs `BorrowMut`) is meaningless in TS anyway -- there's no compile-time enforcement of read-only vs read-write access through a wrapper type.

---

## 4. Weak<T> -- rename or new behavior?

### Finding: Mixed. Some behavior overlaps with WeakRef, some diverges.

JS has built-in `WeakRef<T>` with `deref()` returning `T | undefined`. The spec proposes `Weak<T>` with `upgrade()` returning `Arc<T> | null`.

The key difference: `WeakRef.deref()` returns the raw object if it hasn't been GC'd. `Weak.upgrade()` returns an `Arc<T>` -- incrementing the refcount and creating a new owning handle. These are semantically different:

| | `WeakRef<T>.deref()` | `Weak<T>.upgrade()` |
|--|---|---|
| Returns | `T \| undefined` | `Arc<T> \| null` |
| Keeps object alive? | No (ephemeral) | Yes (new Arc owner) |
| Deterministic? | No (GC-dependent) | Yes (refcount > 0?) |
| Drop semantics | None | Upgraded Arc participates in drop counting |

In the Rust codebase, `Weak<T>` is used in exactly these places:

1. **`WeakEntity(Weak<EntityInner>)`** -- `EntityInner` has no `impl Drop`. This is just a non-preventing reference. `WeakRef<T>` suffices.

2. **`WeakNode(Weak<NodeInner>)`** -- `NodeInner` has no `impl Drop`. Same -- `WeakRef<T>` suffices.

3. **`WeakEntityLiveQuery(Weak<Inner>)`** -- `Inner` **does** have `impl Drop`. And `upgrade()` returns `Option<EntityLiveQuery>` (which wraps `Arc<Inner>`). This is the real case: upgrading creates a new owning handle that participates in the refcount. If upgrade succeeds, the caller now co-owns the inner and its drop won't fire until this new handle is also dropped.

4. **`Weak<PNBackend>` in pn_counter** -- `PNBackend` has no `impl Drop`. Plain `WeakRef` suffices.

5. **`Weak<NodeInner>` in fetch_gap** -- No `impl Drop`. `WeakRef` suffices.

So only case (3) actually needs `Weak<T>` as paired with `Arc<T>`. The others should use native `WeakRef<T>`.

### The determinism problem

`WeakRef.deref()` is GC-dependent -- it may return the object even after all strong references are gone (GC hasn't run yet) or return `undefined` while strong references still exist (spec allows this, though engines don't do it in practice). `Weak<T>.upgrade()` paired with `Arc<T>` is deterministic -- it succeeds if and only if the refcount is > 0.

For `WeakEntityLiveQuery`, determinism matters: the remote subscriber calls `upgrade()` and needs to know definitively whether the LiveQuery is still alive. GC non-determinism could cause:
- `deref()` returns the object after all Arcs are dropped but before GC runs -> subscriber uses a "dead" LiveQuery whose Drop hasn't fired yet
- `deref()` returns undefined while Arcs still exist (unlikely but spec-legal) -> subscriber incorrectly believes LiveQuery is gone

**Verdict**: `Weak<T>` is needed, but only as a companion to `Arc<T>`. It should only be used where `Arc<T>` is used (i.e., the 2 types identified in section 1). For all other Rust `Weak<T>` patterns wrapping non-Drop types, use native `WeakRef<T>`.

The spec should add: "Weak<T> is only used in conjunction with Arc<T>. For Rust Weak<T> where T does not implement Drop, use JS native WeakRef<T>."

---

## 5. Adversarial edge cases

### 5a. Arc<T> prevents garbage collection

Once you wrap something in `Arc<T>` with explicit refcounting, JS's GC can no longer collect the inner even if no JS references exist to any Arc handle -- because the Arc's closure/prevent mechanism might still hold a strong reference internally. Implementation must ensure that when all Arc handles are unreachable (not just not-dropped), the FinalizationRegistry backstop fires to decrement the refcount.

This is subtle: if `Arc` stores the inner in a shared object and each Arc handle holds a reference to that shared object, all handles becoming unreachable means the shared object also becomes unreachable, so GC can collect it. But if any leaked handle prevents collection, the inner leaks too. The FR backstop must handle this.

### 5b. Clone semantics confusion

The spec shows:
```typescript
const handle1 = sub.clone();
```

But in TS, assignment is already reference-copying: `const handle1 = sub` would give you another reference to the same Arc. Is `.clone()` creating a new Arc handle (incrementing refcount), or is assignment sufficient?

If `Arc<T>` is a class instance, then `const handle1 = sub` already shares the same object. You'd need `.clone()` to create a *separate* Arc handle that independently participates in refcounting. But then `const handle1 = sub` creates a reference leak -- two JS references but only one refcount increment.

**This is a fundamental problem**: JS assignment semantics and explicit refcounting are in tension. Every `const x = arc` that doesn't go through `.clone()` is a potential bug. The lint rule must catch bare assignment of Arc values.

### 5c. Transaction is NOT wrapped in Arc (despite `Arc<Self>` in uniffi)

`Transaction` has `impl Drop` but is never wrapped in `Arc` for shared ownership in the core code. The `Arc<Self>` in `uniffi_commit` is a UniFFI binding requirement, not a shared-ownership pattern. Transaction should remain a plain `Disposable` in TS, not `Arc<Transaction>`.

### 5d. Spec says "Rc<T> -> Arc<T>" but Rc is never used

There are zero occurrences of `Rc<` in `ankurah/core/src/`. The mapping rule is correct but vacuous. Not a problem, just noting it.

---

## Summary of recommendations

| Issue | Severity | Recommendation |
|-------|----------|----------------|
| Arc<T> mapping is too broad | **High** | Qualify: only when T: Drop + multi-owner clone. Otherwise plain T. |
| Borrow<T>/BorrowMut<T> are ceremony | **Medium** | Remove. Use `/** @borrowed */` lint annotations instead. |
| Weak<T> vs WeakRef<T> conflation | **Medium** | Weak<T> only with Arc<T>. Use native WeakRef for non-Drop Weak patterns. |
| `.clone()` vs assignment ambiguity | **High** | Spec must address: is bare assignment of Arc a lint error? |
| Generic Arc<T> for 2 use sites | **Low** | Consider whether 2 types justify a generic. Inline refcounting may be simpler. |
| Spec doesn't show field access through Arc | **Medium** | Add examples of how internal code accesses Arc'd fields. |
