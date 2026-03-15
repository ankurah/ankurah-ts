# Adversarial Review: Arc/Weak/Borrow Ownership Spec

**Reviewer**: Adversarial Reviewer
**Date**: 2026-03-14
**Spec reviewed**: `port/ownership.md` (Arc<T>, Weak<T>, Borrow<T>, BorrowMut<T>)

---

## Verdict: The spec is reasonable but oversells Arc's necessity for this codebase

The spec is well-written and the API surface is clean. However, some of the proposed types solve problems that don't currently exist in the TS port, and the spec glosses over a critical ergonomics question that previously sank Arc proposals. Below I try to break each piece.

---

## 1. The `.value` Problem — NOT Solved

**Status: CRITICAL CONCERN**

The spec shows this usage:

```typescript
const sub = Arc.new(new ReactorSubInner());
const handle1 = sub.clone();
handle1.drop();
```

But it never shows how you **access the inner value**. In the Rust codebase, `ReactorSubscription(Arc<ReactorSubInner>)` uses `self.0.subscription_id` — direct field access through Deref. The spec doesn't address what the TS equivalent looks like.

If `Arc<T>` is a wrapper, then every access site becomes `arc.inner.field` or `arc.get().field` or similar. This was the reason Arc was previously rejected — it pollutes every access site.

The spec must answer: **How does code inside `ReactorSubscription` access `ReactorSubInner` fields?** Show a real translated method, not just clone/drop examples.

Looking at the current TS code (`packages/core/src/reactor/subscription.ts`), `ReactorSubscription` stores `inner: ReactorSubInner` as a plain field and accesses `this.inner.subscriptionId`, `this.inner.broadcast`, etc. If this becomes `Arc<ReactorSubInner>`, does every access become `this.inner.get().subscriptionId`? That's exactly the ergonomic tax that was rejected before.

**Possible mitigations the spec should address:**
- If Arc is only used at construction/drop boundaries (not on every field access), say so explicitly and show the pattern.
- If Arc uses a Proxy to transparently forward property access, document the performance and type-safety implications.
- If the answer is "Arc is only for the refcount, the inner is accessed directly," then what's the point of wrapping it?

---

## 2. Arc::clone() Semantics — Underspecified

**Status: MODERATE CONCERN**

In Rust, `Arc::clone()` increments the refcount and returns another `Arc<T>` pointing to the same allocation. The spec shows this correctly. But the spec doesn't address:

**What does the cloned Arc's type look like to TypeScript?** If `arc.clone()` returns `Arc<ReactorSubInner>`, and you need to call methods on `ReactorSubInner`, you still have the wrapper indirection problem. In Rust, `Deref` makes this invisible. TS has no `Deref`.

**Who calls `clone()` in practice?** Looking at the Rust codebase:

- `ReactorSubscription.clone()` — used in `client_relay.rs` (line 215) to store in subscription state, and conceptually when spawning async tasks. But in the TS port, `client_relay` is Phase 1 out-of-scope (the relay is stubbed to `null`). The only current clone site is `EntityLiveQuery.new()` where Rust does `let me2 = me.clone()` before `crate::task::spawn`. The TS port handles this with `void me.activate(1).then(...)` — no clone needed because JS closures capture by reference, not by move.

- `Reactor.clone()` — used because Rust needs `self.clone()` to move into `ReactorSubInner`. TS already avoids this by using a callback `unsubscribeFn` instead of storing a Reactor reference.

- `Subscription.clone()` — used in `reactor.rs:355` to clone the subscription before calling `evaluate_changes` across an async boundary. TS handles this by directly calling `subscription.evaluateChanges(candidates)` — no clone needed.

**In every case where Rust clones an Arc, the TS port already works without cloning.** This is because JS closures capture references (not values) and JS is single-threaded (no need to move owned data into spawned tasks).

---

## 3. Performance — Acceptable but Unnecessary

**Status: LOW CONCERN**

Runtime refcounting overhead per clone/drop is negligible in absolute terms (increment/decrement a number). The spec correctly notes this.

However, the more relevant performance question is: **how many Arc wrappers exist per session, and how often are they accessed?** In a typical ankurah session:

- 1 `Reactor` (currently plain object)
- N `ReactorSubscription`s (one per LiveQuery, typically 1-10)
- N `Subscription`s (one per ReactorSubscription)
- 1 `Node` (currently plain object)
- N `EntityLiveQuery`s (one per active query, typically 1-10)

This is a small number. The overhead of Arc wrapping is not a performance problem. But it's also not solving a performance problem — it's allegedly solving a correctness problem. See section 5.

---

## 4. Interaction with `using` — Correct but Subtle

**Status: LOW CONCERN**

The spec correctly describes that `using arc = ...` drops one Arc handle at block exit. If the Arc was cloned inside the block, the clone survives. This is correct behavior.

However, there's a subtle issue the spec should address: **What happens when you `using` an Arc and also store a clone?**

```typescript
let escaped: Arc<Foo>;
{
    using arc = Arc.new(new Foo());
    escaped = arc.clone();
} // arc dropped, but escaped still holds a reference
// escaped.get().someMethod() — still works, inner not dropped
escaped.drop(); // NOW inner drops
```

This is correct! But it's also exactly the pattern that `using` is supposed to prevent — values escaping their scope. The `assertNotDropped()` guard on Drop catches the plain case (`let bar; { using foo = ...; bar = foo; }`) because `foo` IS dropped. But with Arc, the escape is intentional and valid. This means **Arc partially undermines the safety guarantees of `using`** — a cloned Arc can escape a `using` block without triggering any diagnostic.

This isn't necessarily wrong, but the spec should acknowledge it as a known interaction.

---

## 5. Do We ACTUALLY Have Bugs? — No Evidence Found

**Status: CRITICAL CONCERN**

I searched the entire TS codebase for a concrete scenario where two holders of the same inner object cause premature cleanup. Here's what I found:

### ReactorSubscription / ReactorSubInner
In Rust, `ReactorSubscription(Arc<ReactorSubInner>)` uses Arc because the subscription can be cloned and given to multiple owners (LiveQuery holds one, client_relay holds another). When the last owner drops, `ReactorSubInner::drop()` calls `reactor.unsubscribe()`.

In the TS port, `ReactorSubscription` is NOT cloned anywhere. It's created in `Reactor.subscribe()`, stored in `EntityLiveQuery`, and dropped when `EntityLiveQuery.drop()` is called. There is exactly one owner. The `client_relay` (which would be the second owner in Rust) is stubbed to `null` in Phase 1.

**No premature cleanup bug exists here.**

### EntityLiveQuery / Inner
In Rust, `EntityLiveQuery(Arc<Inner>)` uses Arc because it's cloned when spawning async tasks (`let me2 = me.clone(); task::spawn(async move { me2.activate(...) })`). The clone keeps the inner alive while the task runs.

In TS, `void me.activate(1).then(...)` captures `me` by reference in a closure. JS garbage collection keeps `me` alive as long as the closure exists (which is as long as the promise is pending). **No Arc needed — JS reference semantics already prevent premature cleanup.**

### Reactor / ReactorInner
In Rust, `Reactor(Arc<ReactorInner>)` uses Arc because the Reactor is cloned into `ReactorSubInner` (so the subscription can call back to the reactor on drop). The TS port avoids this entirely by using a callback function `unsubscribeFn` instead of storing a Reactor reference.

**No premature cleanup bug exists here.**

### Node / NodeInner
In Rust, `Node(Arc<NodeInner>)` uses Arc because the Node is cloned into various contexts (transactions, LiveQueries, peer connections). In TS, `Node` is a plain class, and JS reference counting (GC) keeps it alive as long as anything references it.

**No premature cleanup bug exists here.**

### WeakEntityLiveQuery
Uses `WeakRef<EntityLiveQuery>` correctly — this maps to Rust's `Weak<Inner>`. The weak reference pattern works natively in JS via `WeakRef` without needing Arc.

### Summary
**Every place Rust uses Arc in the reactor/subscription/livequery code, the TS port already handles the same concern through JS's native reference semantics (GC + closures).** Arc would be solving a theoretical problem that JS's runtime already prevents.

---

## 6. Borrow<T> / BorrowMut<T> — Useful for Lint, Low Cost

**Status: ACCEPTABLE**

These are marker types with no runtime behavior. They exist purely to help the eslint plugin distinguish "I own this field and must drop it" from "I'm borrowing this field and must NOT drop it." This is genuinely useful — without it, the linter can't tell whether `this.reactor` in `EntityLiveQuery` should be dropped in `drop()` or not.

The cost is near zero (a type wrapper), and the benefit is real (lint accuracy). No objection.

---

## 7. Weak<T> — Redundant with WeakRef

**Status: MODERATE CONCERN**

The spec proposes `Weak<T>` with `upgrade() -> Arc<T> | null`. But JS already has `WeakRef<T>` with `deref() -> T | undefined`. The TS codebase already uses `WeakRef` directly (see `WeakEntityLiveQuery`, `WeakEntitySet`, `QueryGapFetcher`).

If we adopt `Arc<T>`, then `Weak<T>` makes sense as its companion (upgrade returns an Arc, not a raw reference). But if Arc isn't needed (see section 5), then `Weak<T>` adds a layer of indirection over `WeakRef` for no benefit.

---

## Recommendations

1. **Do not adopt Arc<T> for Phase 1.** No concrete bugs justify it, and JS reference semantics already handle the cases where Rust needs Arc. The `.value` access ergonomic tax is real and unresolved.

2. **Do adopt Borrow<T> / BorrowMut<T>.** These are low-cost marker types that improve lint accuracy. They answer a real question ("should this field be dropped?") that the current code answers only via comments.

3. **Defer Weak<T>.** Continue using `WeakRef` directly. If Arc is adopted later (e.g., when client_relay is ported and multiple owners of ReactorSubscription emerge), add Weak<T> at that time.

4. **If Arc is adopted later**, the spec MUST first resolve the `.value` access pattern. Show a real translated method from `ReactorSubscription` or `EntityLiveQuery` with and without Arc, and demonstrate that the ergonomic cost is acceptable. The Rust codebase's Deref magic is doing a LOT of heavy lifting that has no TS equivalent.

5. **Document when Arc will become necessary.** The trigger is likely Phase 2's `client_relay` port, where `ReactorSubscription` genuinely has two owners. Add a note to the spec: "Arc becomes necessary when [specific feature] is ported, because [specific shared-ownership scenario]."
