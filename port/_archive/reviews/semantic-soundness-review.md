# Semantic Soundness Review: memory-model.md

**Reviewer**: Semantic Soundness Reviewer Agent
**Date**: 2026-03-12
**Spec reviewed**: `/Users/daniel/ak/ankurah-ts/specs/memory-model.md`
**Verdict**: The spec is largely honest and well-reasoned, with two unsound mappings and several gaps in the Inherent Limitations section.

---

## Methodology

For each mapping claimed in the spec, I:
1. Read the Rust source to identify the exact guarantee provided
2. Read the TS implementation to verify the claimed mapping
3. Attempted to construct an adversarial scenario that breaks it
4. Assessed whether the gap matters for ankurah's specific usage

---

## 1. Drop -> Disposable (Section 2)

**Verdict: MOSTLY SOUND**

**Rust guarantees**: `drop()` is called deterministically when a value leaves scope. The compiler enforces this -- there is no way to skip it (barring `mem::forget`, which is safe but discouraged).

**TS provides**: `Disposable` base class with `onDispose()`, `[Symbol.dispose]()` for `using`, and `FinalizationRegistry` as a diagnostic safety net. Disposal is idempotent.

**Gap analysis**: The core gap is that TS disposal is voluntary. The `using` keyword provides scope-based cleanup analogous to Rust's Drop, but:
- Developers can forget `using` and forget `.dispose()`
- `FinalizationRegistry` may never fire (per spec)
- The escape hatch (`let bar; { using foo = ..; bar = foo; }`) leaks a disposed reference

**Breaking scenario**:
```typescript
const sub = reactor.subscribe(); // no `using`, no try/finally
// ... exception thrown ...
// sub is never disposed, subscription leaks forever
// FR may or may not fire, may or may not clean up
```

**Does it matter?** Yes, but the spec is honest about this. The two-tier FR policy (crash for correctness-critical, warn for hygiene) is the right mitigation. The `assertNotDisposed()` guard catches use-after-dispose at runtime. The residual risk is orphaned subscriptions from forgotten `dispose()` calls, which is a resource leak, not a correctness issue for the hygiene-tier types.

**One issue**: The spec says `FinalizationRegistry` "warns or crashes" but the actual `leakRegistry` in `disposable.ts:37-43` only does `console.error`. There is no `queueMicrotask(() => { throw ... })` path for correctness-critical types. The spec claims two tiers but the implementation has one. This needs to be reconciled -- either the implementation needs the hard-crash path for correctness-critical types, or the spec needs to acknowledge that all FR callbacks are warn-only in the current implementation. **However**, this is moot for `ResultSetWrite` because it uses `RefCell.withMut()` and never goes through `Disposable` or `FinalizationRegistry` at all (which is correct).

---

## 2. ResultSetWrite Drop -> RefCell.withMut() (Section 3b, 6)

**Verdict: MOSTLY SOUND**

**Rust guarantees**: `ResultSetWrite` holds a `MutexGuard` and broadcasts on `Drop` (line 335-343 of `resultset.rs`). The guard is released *before* the broadcast (`drop(self.guard.take())`). The mutex ensures exclusive access during mutation. Drop is guaranteed by the compiler.

**TS provides**: `RefCell<T>` with `withMut()` using `try/finally` to guarantee `onMutRelease` fires. Re-entrancy detection prevents nested mutable borrows.

**Gap analysis**: The `try/finally` in `withMut()` (line 228-233 of `disposable.ts`) does guarantee that `onMutRelease` fires, matching Rust's Drop guarantee. The re-entrancy check mirrors `RefCell` panic behavior.

**But the spec describes one thing and the code does another.** The actual `resultset.ts` still has the old `ResultSetWrite` class with `write()/done()` pattern (lines 159-384). It does NOT use `RefCell.withMut()`. The `EntityResultSet.write()` method (line 492) returns a `ResultSetWrite` object that the caller must manually call `.done()` on. Every usage in `subscription_state.ts` follows the `write()/done()` pattern:

```typescript
// subscription_state.ts line 328, 368
const rwResultset = queryState.resultset.write();
// ... mutations ...
rwResultset.done();  // manual call, no try/finally
```

This is exactly the pattern the spec calls "the single most dangerous pattern in the codebase" (Section 3b). If an exception is thrown between `write()` and `done()`, the broadcast never fires and observers silently see stale data. **The spec's recommended RefCell pattern is not implemented.**

**Breaking scenario**:
```typescript
const rw = resultset.write();
rw.add(entity);
throw new Error("oops"); // broadcast never fires
// rw.done() never called
// observers see stale data -- SILENT CORRECTNESS BUG
```

**Does it matter?** Yes, critically. This is the single most important correctness property in the system. The spec correctly identifies this as correctness-critical and correctly prescribes `RefCell.withMut()`, but the implementation has not caught up. Every `write()/done()` call site is a potential silent-stale-data bug.

**Recommendation**: Migrate all `write()/done()` call sites to use `RefCell.withMut()` as the spec prescribes, or at minimum wrap every `write()/done()` pair in `try/finally`.

---

## 3. Transaction Drop -> Disposable/using (Section 3a)

**Verdict: MOSTLY SOUND**

**Rust guarantees**: `Transaction::drop()` sets `alive = false` (line 127-130 of `transaction.rs`). `commit(self)` and `rollback(self)` both consume `self`, so after either call the transaction cannot be used again (enforced at compile time by move semantics).

**TS provides**: `Transaction` class with `commit()` and `rollback()` methods. The `alive` flag is set to `false` by `commit()` (via `commitLocalTrx` at `node.ts:265`) and `rollback()` (at `transaction.ts:216`).

**Gap analysis -- the spec claims Transaction extends Disposable but it doesn't.** Looking at `transaction.ts`, the `Transaction` class does NOT extend `Disposable`, does NOT have `[Symbol.dispose]()`, and does NOT register with `FinalizationRegistry`. The spec (Section 3a) says:

> Transaction MUST extend Disposable (or use DisposeGuard)
> `[Symbol.dispose]()` MUST call `rollback()` if neither commit nor rollback has been called

None of this is implemented. The spec's requirements are correct but unimplemented.

**Breaking scenario -- the spec's main concern**:
```typescript
const trx = await node.begin();
const record = await trx.create(MyModel, data);
// exception thrown here -- no commit, no rollback
// trx.alive.value remains true
// MutableBorrow references can still mutate (alive check passes)
// ... until GC runs, which may be never
```

**The alive gap is real.** Without `Disposable`/`using`, if an exception prevents both `commit()` and `rollback()`, the alive flag stays `true` indefinitely. Any `MutableBorrow` references obtained from the transaction can continue to mutate entity forks that will never be committed. This is a resource leak and potentially confusing, but per the user's guidance ("who cares, nobody is hurt"), uncommitted fork mutations are harmless -- they affect fork entities that are never persisted.

**Does it matter?** Low severity per user's stated position. The mutations affect only fork entities that will never be committed. But the spec's requirements (extend Disposable, FR diagnostic) are correct for defense-in-depth and should be implemented.

---

## 4. Mutex/RwLock -> Eliminated (Section 2, 13)

**Verdict: SOUND (for std::sync::Mutex), MOSTLY SOUND (for tokio::sync::Mutex)**

**Rust guarantees**: `std::sync::Mutex` prevents data races between threads. `tokio::sync::Mutex` serializes async operations across `.await` points.

**TS provides**: For `std::sync::Mutex`, plain fields (correct -- JS is single-threaded within a synchronous block). For `tokio::sync::Mutex`, `PromiseMutex` in `reactor/index.ts`.

**Gap analysis for std::sync::Mutex elimination**: Sound. Within a synchronous JS execution slice, no interleaving can occur. The spec correctly identifies this.

**Gap analysis for tokio::sync::Mutex**: The `PromiseMutex` in `reactor/index.ts:122-135` correctly serializes `notifyChange` calls. However, the spec (Section 13) honestly documents two **missing** PromiseMutex usages:
1. WatcherSet mutation from `fillGapsAndNotify` (fire-and-forget async)
2. SystemManager lifecycle ops

**Breaking scenario for the WatcherSet gap**:
```typescript
// In subscription_state.ts line 532:
this.fillGapsAndNotify(updateItems, gapsToFill);
// This is fire-and-forget (no await)
// It mutates the resultset via write()/done()
// Meanwhile, another notifyChange could run and see inconsistent watcher state
```

The spec correctly flags this as a gap. The `fillGapsAndNotify` call at `subscription_state.ts:532` is not awaited, meaning it runs outside the `notifyLock` in `reactor/index.ts`. This matches the Rust behavior (which uses `task::spawn` outside the lock), but the Rust code protects the `WatcherSet` with its own `std::sync::Mutex`. In TS, the `WatcherSet` is a plain object with no synchronization, so concurrent async operations could interleave.

**Does it matter?** Yes, this is a real race condition. But the spec is honest about it (Section 13, "MISSING" annotations).

---

## 5. RefCell<T> -> RefCell<T> class (Section 6)

**Verdict: MOSTLY SOUND**

**Rust guarantees**: `RefCell<T>` panics on double-mutable-borrow at runtime. The borrow checker prevents the callback from escaping the reference (lifetimes enforce this at compile time).

**TS provides**: `RefCell<T>` class in `disposable.ts` with runtime borrow-state tracking. `withMut()` throws on re-entrant mutable borrows. `withRef()` allows multiple shared borrows. `try/finally` ensures borrow state is always released.

**Gap analysis**: The borrow tracking is correct and mirrors Rust's `RefCell` semantics faithfully. The state machine (`not_borrowed` / `shared(N)` / `mut_borrowed`) correctly prevents:
- Mutable borrow while shared borrows exist
- Mutable borrow while another mutable borrow exists
- Shared borrow while a mutable borrow exists

**The reference escape problem is real and correctly documented.** The spec (Section 6, constraint 2) and Section 15b correctly identify that JS cannot prevent:
```typescript
let leaked: T;
cell.withMut((value) => { leaked = value; });
// leaked is now a live reference that bypasses all borrow tracking
```

**Additional gap not in the spec -- async callbacks**:
The spec correctly notes "No async inside `withMut()`" (Section 6, constraint 1), but the TS `RefCell` does not actually detect or prevent this. If someone passes an `async` function to `withMut()`, the `finally` block runs at the first `await`, releasing the borrow while the callback is still running:

```typescript
cell.withMut(async (value) => {
    value.doSomething(); // borrow is active
    await somePromise;   // finally runs here, borrow released
    value.doMore();      // borrow tracking says "not borrowed" but we're still mutating
});
```

The spec documents this but the code has no runtime guard against it (no check that `fn` returns a non-Promise value). This is an inherent limitation.

**Does it matter?** The escape hatch and async issues are inherent JS limitations. The spec is honest about both. For ankurah's usage (ResultSetWrite-like patterns), the callbacks are short synchronous lambdas, making escape structurally unlikely.

---

## 6. Arc<T> -> Plain reference (Section 2)

**Verdict: SOUND**

**Rust guarantees**: `Arc<T>` provides shared ownership with reference counting. The value is dropped when the last `Arc` is released.

**TS provides**: Plain JS references. The GC keeps the object alive as long as any reference exists.

**Gap analysis**: This is semantically equivalent. JS GC provides "lives as long as any reference exists" which is exactly what `Arc<T>` provides. The only difference is that Rust `Arc` drops deterministically when the last reference goes away, while JS GC is non-deterministic. But since `Arc` itself has no `Drop` side effects (it's the *inner* type's Drop that matters), this mapping is sound.

**Does it matter?** No. This is one of the cleanest mappings in the spec.

---

## 7. Weak<T> -> WeakRef<T> (Section 7)

**Verdict: SOUND**

**Rust guarantees**: `Weak::upgrade()` returns `None` immediately when the last `Arc` is dropped.

**TS provides**: `WeakRef<T>` with `deref()` returning `undefined` when the target is collected.

**Gap analysis**: The spec correctly documents the timing difference (Section 7): JS `WeakRef.deref()` may return the object within the same microtask turn even after all strong references are gone. This makes JS `WeakRef` "stronger" (more permissive) than Rust `Weak` within a synchronous slice.

**The implementation matches the spec.** `WeakEntityLiveQuery` at `livequery.ts:490-505` correctly uses `WeakRef` and handles the `undefined` case. `WeakEntitySet` at `entity.ts:478-555` correctly uses `WeakRef` with `FinalizationRegistry` for map cleanup.

**Does it matter?** No. The "bonus access" within a microtask is harmless -- the TS code just succeeds where Rust would fail. All call sites handle the `undefined` case.

---

## 8. Lifetime params -> alive flags (Section 14)

**Verdict: UNSOUND**

**Rust guarantees**: `MutableBorrow<'rec, T>` cannot outlive its creating transaction. The borrow checker prevents this at compile time. After `trx.commit(self)` or `trx.rollback(self)`, the transaction is moved and all borrows are invalidated.

**TS provides**: The `alive` flag on the `Transaction` object, checked by `Entity.isWritable()`.

**Gap analysis -- the checks are missing.** The spec (Section 14) requires:
- `Transaction.create()` MUST check `this.alive.value` -- **NOT IMPLEMENTED** (see `transaction.ts:112-125`, no alive check)
- `Transaction.get()` MUST check `this.alive.value` -- **NOT IMPLEMENTED** (see `transaction.ts:139-161`, no alive check)
- `Transaction.edit()` MUST check `this.alive.value` -- **NOT IMPLEMENTED** (see `transaction.ts:174-187`, no alive check)
- `LWW.set()` MUST check `entity.isWritable()` -- Cannot verify (property value types not fully implemented)
- `YrsString.insert()` MUST check `entity.isWritable()` -- Cannot verify

**Breaking scenario**:
```typescript
const trx = await node.begin();
const record = await trx.create(MyModel, { name: "test" });
await trx.commit(); // alive set to false

// But Transaction methods don't check alive:
const record2 = await trx.create(MyModel, { name: "ghost" });
// This succeeds! Creates an entity in a dead transaction.
// The entity will never be committed (trx is already committed).
// This is confusing but not dangerous -- the fork is orphaned.
```

More seriously:
```typescript
const trx = await node.begin();
const borrow = await trx.get(MyModel, someId);
await trx.commit(); // alive = false

// borrow.inner still exists and may still have active handles
// If LWW.set() doesn't check isWritable(), mutations on the fork succeed silently
// The mutations affect a fork entity that will never be committed
```

**Does it matter?** The mutations affect orphaned fork entities, so no data corruption occurs. But the lack of alive checks means developers get no error feedback when using a committed transaction, which is a developer experience issue and could mask bugs. The spec correctly identifies these as requirements but they are unimplemented.

---

## 9. AtomicBool/AtomicU32 -> plain boolean/number (Section 2)

**Verdict: SOUND**

**Rust guarantees**: Atomics provide thread-safe read-modify-write operations with ordering guarantees.

**TS provides**: Plain `boolean` and `number` fields.

**Gap analysis**: JS is single-threaded within a synchronous block. There are no concurrent readers/writers within a synchronous slice. At `await` boundaries, the event loop yields but only one task runs at a time. This mapping is correct.

**Does it matter?** No. This is completely sound for single-threaded JS.

---

## 10. Vicarious RAII (Section 10)

**Verdict: MOSTLY SOUND (spec is correct, implementation is incomplete)**

**Rust guarantees**: Drop cascades automatically through owned fields. A struct that owns a field with `impl Drop` will see that field's `drop()` called.

**TS provides**: The spec correctly requires that each owner's `onDispose()` must call `dispose()` on owned Disposable fields.

**Gap analysis**: The spec's classification tables (Section 10) are accurate. I verified:
- `EntityLiveQuery` owns `ReactorSubscription` which has `impl Drop` -> spec correctly marks as vicarious RAII
- `LiveQuery<R>` owns `EntityLiveQuery` -> correctly marked as transitive
- `ReactorSubscription` owns `ReactorSubInner` which has `impl Drop` -> correctly marked

**Implementation check**: `EntityLiveQuery.dispose()` at `livequery.ts:464-470` does call `this.subscription.dispose()`. `LiveQuery.dispose()` at `livequery.ts:680-682` delegates to `this.inner.dispose()`. The chain is correct.

**However**, `ReactorSubscription` at `reactor/subscription.ts:77` does NOT extend `Disposable`. It has a manual `dispose()` method and `[Symbol.dispose]()` but no `FinalizationRegistry` registration. If a `ReactorSubscription` is forgotten, no diagnostic warning fires.

Similarly, `EntityLiveQuery` does NOT extend `Disposable`. It has its own `FinalizationRegistry` at `livequery.ts:39-44` but it's a stub (`void queryId` -- does nothing).

**Does it matter?** These are resource-hygiene types, so the impact is leaked subscriptions, not correctness bugs. But the spec's checklist ("Extends Disposable with a descriptive label") is not met for these types.

---

## 11. PromiseMutex (Section 13)

**Verdict: MOSTLY SOUND**

**Rust guarantees**: `tokio::sync::Mutex<()>` serializes async operations. The guard is held across `.await` points.

**TS provides**: `PromiseMutex` class at `reactor/index.ts:122-135` using a promise chain.

**Gap analysis**: The PromiseMutex correctly serializes operations: each caller awaits the previous promise before running, and releases when done (via `finally`). This mirrors `tokio::sync::Mutex<()>` semantics.

**However, the implementation differs from the spec.** The spec (Section 13) shows:
```typescript
async run<T>(fn: () => Promise<T>): Promise<T> { ... }
```

The actual implementation uses `acquire()/release()`:
```typescript
async acquire(): Promise<() => void> { ... }
```

This is a stylistic difference, not a semantic one. Both achieve serialization. The `acquire/release` pattern is actually more flexible but also more dangerous (caller could forget to call `release()`). The `run()` pattern from the spec is safer because `try/finally` is built in. The actual `notifyChange` at `reactor/index.ts:434-479` does use `try/finally` around the `release()`, so it's safe in practice.

**Does it matter?** The serialization is correct. The stylistic difference is minor.

---

## 12. Disposal Order (Section 12)

**Verdict: SOUND (for the spec claim; unverifiable in practice)**

**Rust guarantees**: Fields are dropped in declaration order (which is effectively reverse construction order for most patterns).

**TS provides**: The spec recommends disposing in reverse construction order.

**Gap analysis**: This is a recommendation, not an enforced guarantee. In the current codebase, the few `onDispose()` implementations I found dispose fields in a single order. Since most Disposable types own only one or two Disposable fields, order rarely matters.

---

## 13. The `using` Escape Hatch (Section 12)

**Verdict: SOUND (spec correctly identifies the problem)**

The spec correctly identifies the escape hatch:
```typescript
let leaked: MySubscription;
{
    using sub = reactor.subscribe();
    leaked = sub; // BAD
}
// leaked is disposed
```

The `assertNotDisposed()` guard on public methods is the correct mitigation. This is an inherent JS limitation that cannot be solved at the language level.

---

## 14. Completeness of Inherent Limitations (Section 15)

**Verdict: INCOMPLETE -- missing two important limitations**

Section 15 lists four limitations (15a-15d). These are all correctly described. However, two additional inherent limitations are missing:

### Missing: 15e. No move semantics -- Transaction double-use

Rust's `commit(self)` and `rollback(self)` consume the Transaction, making it impossible to use after either call. TS has no move semantics, so nothing prevents calling `commit()` and then `rollback()` (or `create()` after `commit()`). The `alive` flag is the mitigation, but even if alive checks were implemented on all methods, the fundamental issue is that TS developers get a runtime error instead of a compile-time error. This is a distinct limitation from 15a (which focuses on MutableBorrow lifetimes, not Transaction consumption).

### Missing: 15f. No async callback detection in RefCell.withMut()

The spec mentions this in Section 6 (constraint 1) but does not include it in Section 15. Passing an `async` function to `withMut()` silently breaks borrow tracking because `finally` runs at the first `await`. This is an inherent limitation: TS cannot distinguish `() => R` from `() => Promise<R>` at the type level (since `Promise<R>` is a valid `R`). A runtime check (`result instanceof Promise ? throw : return`) would catch it but break legitimate use cases where the callback returns a Promise-like value.

---

## Summary Table

| # | Mapping | Spec Section | Verdict | Key Issue |
|---|---------|-------------|---------|-----------|
| 1 | Drop -> Disposable | 2, 4 | MOSTLY SOUND | Voluntary disposal; FR only warns, never crashes |
| 2 | ResultSetWrite Drop -> RefCell.withMut | 3b, 6 | MOSTLY SOUND | **Spec prescribes RefCell but code still uses write()/done()** |
| 3 | Transaction Drop -> Disposable/using | 3a | MOSTLY SOUND | Transaction does not extend Disposable; no FR registration |
| 4 | Mutex -> eliminated / PromiseMutex | 2, 13 | SOUND / MOSTLY SOUND | Missing PromiseMutex for WatcherSet gap fill |
| 5 | RefCell -> RefCell class | 6 | MOSTLY SOUND | Reference escape inherent; no async detection |
| 6 | Arc -> plain reference | 2 | SOUND | Clean mapping |
| 7 | Weak -> WeakRef | 7 | SOUND | Timing difference documented and harmless |
| 8 | Lifetimes -> alive flags | 14 | UNSOUND | **Alive checks not implemented on Transaction methods** |
| 9 | Atomics -> plain fields | 2 | SOUND | Single-threaded JS |
| 10 | Vicarious RAII | 10 | MOSTLY SOUND | Classification correct; FR registration incomplete |
| 11 | PromiseMutex | 13 | MOSTLY SOUND | Works but uses acquire/release vs spec's run() |
| 12 | Disposal order | 12 | SOUND | Recommendation, not enforcement |
| 13 | using escape hatch | 12 | SOUND | Correctly identified, assertNotDisposed mitigates |

---

## Critical Findings (Action Required)

### 1. ResultSetWrite still uses write()/done() -- CORRECTNESS RISK

The spec (Section 3b) explicitly says "Do NOT create a long-lived ResultSetWrite object" and prescribes `RefCell.withMut()`, but the entire codebase still uses the old `write()/done()` pattern. Every call site in `subscription_state.ts` (lines 328, 368, 464, 473, 669, 745) is vulnerable to exception-induced stale data.

**Files affected**: `resultset.ts`, `subscription_state.ts`

### 2. Transaction has no alive checks -- DEVELOPER EXPERIENCE

`Transaction.create()`, `get()`, and `edit()` do not check `this.alive.value`. Using a committed/rolled-back transaction does not throw. The spec requires these checks (Section 14) but they are unimplemented.

**File affected**: `transaction.ts`

### 3. Section 15 is incomplete

Missing inherent limitations: no move semantics (Transaction double-use), no async callback detection in `withMut()`.

---

## Conclusion

The spec is well-structured, honest about most limitations, and correctly identifies the severity classification for each type. The main issues are:

1. **Spec-implementation divergence**: The spec prescribes patterns (RefCell for ResultSetWrite, Disposable for Transaction, alive checks) that the implementation hasn't caught up to. The spec is *correct* -- the implementation needs to match it.

2. **The Inherent Limitations section is 80% complete** but missing two limitations that are mentioned elsewhere in the spec but not collected in Section 15.

3. **No mapping is fundamentally broken** in a way that the spec denies. The spec is honest about its gaps. The two UNSOUND/divergence findings are cases where the spec says "MUST" but the code doesn't yet comply -- not cases where the spec claims something false.
