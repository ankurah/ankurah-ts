# Compliance Assessment: ankurah-ts vs Port Runbook + Ownership Spec

**Date**: 2026-03-14
**Scope**: `packages/core/src/` and `packages/signals/src/`
**Documents audited against**: port-runbook.md, ownership.md, ownership/provided-types.md, translation-rules.md

---

## Priority 1: Correctness-Critical Ownership Violations

### 1.1 Transaction does not extend Disposable and lacks Symbol.dispose

**File**: `packages/core/src/transaction.ts`
**Lines**: 27-218
**What's wrong**: Transaction has `impl Drop` in Rust (sets `alive = false`). The TS class is a plain class with no Disposable base, no Symbol.dispose, no FinalizationRegistry registration, and no assertNotDisposed() guards.
**What the fix is**: Transaction should `extends Disposable`. `onDispose()` should call `this.rollback()` (set alive to false). `commit()` and `rollback()` should call `this.assertNotDisposed()` or check alive explicitly. Add `[Symbol.dispose]` so `using trx = ctx.begin()` auto-rollbacks on scope exit.
**Runbook item**: #8 (Add Symbol.dispose to Transaction)

### 1.2 Transaction.create/get/edit missing alive checks

**File**: `packages/core/src/transaction.ts`
**Lines**: 112-187
**What's wrong**: `create()`, `get()`, and `edit()` do not check `this.alive.value` before performing mutations. In Rust, the lifetime system prevents use after `commit(self)`/`rollback(self)` because `self` is moved. In TS there is no move semantics; the alive flag must be checked at runtime.
**What the fix is**: Add `if (!this.alive.value) throw MutationError.general(new Error('Transaction already committed or rolled back'));` at the top of `create()`, `get()`, and `edit()`.
**Runbook item**: #7 (Add alive checks)

### 1.3 ResultSetWrite uses write()/done() pattern instead of Disposable guard

**File**: `packages/core/src/resultset.ts`
**Lines**: 159-384
**What's wrong**: `ResultSetWrite` is the most critical ownership violation per the runbook. In Rust, `ResultSetWrite` is a `MutexGuard` whose `Drop` impl broadcasts changes. In TS, callers must manually call `done()` — if they forget, changes are silently lost. Every call site does `const rw = resultset.write(); ... rw.done();` with no try/finally protection.
**What the fix is**: `ResultSetWrite` should `extends Disposable`. `onDispose()` should do what `done()` does (broadcast if changed). All call sites should use `using rw = resultset.write();` and remove explicit `done()` calls.
**Runbook item**: #5 (ResultSetWrite -> Disposable guard with using)

### 1.4 ResultSetWrite call sites missing try/finally (silent data loss risk)

**Files & lines** (all in `packages/core/src/reactor/subscription_state.ts`):
- Line 328-368: `updateQuery()` — `rw.done()` at line 368 is not protected by try/finally. If an exception occurs during `markAllDirty()`, `retainDirty()`, etc., `done()` is never called and the broadcast is silently lost.
- Line 464-466: `evaluateChanges()` — `rw.add(); rw.done();` with no try/finally
- Line 472-474: `evaluateChanges()` — `rw.remove(); rw.done();` with no try/finally
- Line 669-679: `processGapFillEntities()` — `rw.add(); rw.done();` with no try/finally
- Line 745-759: `processGapFill()` — same pattern

**What the fix is**: Once ResultSetWrite extends Disposable, all these become `using rw = ...;` and the problem disappears. Until then, every call site needs try/finally wrapping.

---

## Priority 2: Correctness-Critical (Types Not Wired to Disposable)

### 2.1 EntityLiveQuery has ad-hoc dispose() instead of extending Disposable

**File**: `packages/core/src/livequery.ts`
**Lines**: 97-478
**What's wrong**: EntityLiveQuery has `impl Drop` in Rust. The TS version has manual `dispose()` and `[Symbol.dispose]()` methods, plus a separate FinalizationRegistry. But it does not extend Disposable, so it lacks:
- `assertNotDisposed()` guards on public methods
- Idempotent disposal (the FR is separate from the disposal state)
- Consistent FR behavior (the existing FR captures `queryId` but doesn't match the Disposable pattern)
**What the fix is**: Extend Disposable. Move cleanup into `onDispose()`. Remove the separate `liveQueryRegistry` and rely on Disposable's built-in FR. Add `assertNotDisposed()` at the top of `waitInitialized()`, `updateSelection()`, `peek()`, `get()`, `subscribe()`.
**Runbook item**: #6 (Wire existing types to Disposable)

### 2.2 LiveQuery<V> has ad-hoc dispose() instead of extending Disposable

**File**: `packages/core/src/livequery.ts`
**Lines**: 553-687
**What's wrong**: Same pattern as EntityLiveQuery — has `dispose()` and `[Symbol.dispose]()` but does not extend Disposable. No assertNotDisposed() on public methods.
**What the fix is**: Extend Disposable. `onDispose()` calls `this.inner.dispose()`. Add assertNotDisposed() to `get()`, `peek()`, `subscribe()`, `listen()`.
**Runbook item**: #6

### 2.3 ReactorSubscription has ad-hoc dispose() instead of extending Disposable

**File**: `packages/core/src/reactor/subscription.ts`
**Lines**: 77-162
**What's wrong**: ReactorSubscription and ReactorSubInner have manual dispose() methods but don't extend Disposable. No FR registration, no assertNotDisposed() on methods.
**What the fix is**: ReactorSubscription should extend Disposable. Inner cleanup moves to onDispose(). Add assertNotDisposed() on `listen()`, `subscribe()`, `id()`.
**Runbook item**: #6

### 2.4 ListenerGuard (signals) has dispose() but no Disposable/FR

**File**: `packages/signals/src/broadcast.ts`
**Lines**: 52-76
**What's wrong**: `ListenerGuard` has `dispose()` but does not extend Disposable and has no FinalizationRegistry registration. In Rust, `ListenerGuard` implements `Drop`. A forgotten `dispose()` causes a listener leak with no diagnostic.
**What the fix is**: Should extend Disposable (from core's disposable.ts or a shared location). `onDispose()` does the unsubscribe. FR catches leaked guards.
**Runbook item**: #6

### 2.5 SubscriptionGuard (signals) has dispose() but no Disposable/FR

**File**: `packages/signals/src/porcelain/subscribe.ts`
**Lines**: 23-37
**What's wrong**: Same as ListenerGuard — `dispose()` without Disposable base or FR. Leaked guards silently keep subscriptions alive.
**What the fix is**: Should extend Disposable.
**Runbook item**: #6

### 2.6 ListenerGuard (signal/index.ts) wrapper has dispose() but no Disposable/FR

**File**: `packages/signals/src/signal/index.ts`
**Lines**: 24-43
**What's wrong**: The `signal::ListenerGuard` wrapper class has `dispose()` but does not extend Disposable. It wraps a `broadcast::ListenerGuard` which also lacks Disposable.
**What the fix is**: Should extend Disposable.
**Runbook item**: #6

---

## Priority 3: Resource Hygiene (Missed Cleanup = Waste, Not Wrong Data)

### 3.1 Disposable.ts missing Mutex<T>, MutexGuard<T>, Ref<T>, RefMut<T>

**File**: `packages/core/src/disposable.ts`
**Lines**: 1-261
**What's wrong**: The ownership spec (ownership.md) specifies Mutex<T>/MutexGuard<T> and RefCell<T> with borrow()/borrow_mut() returning Disposable guards (Ref<T>/RefMut<T>). The current implementation:
- Has no Mutex<T> or MutexGuard<T> types at all
- RefCell exists but uses closure-based withMut()/withRef() API, not the borrow()/borrow_mut() guard API specified in provided-types.md
**What the fix is**:
1. Add Mutex<T> class with `lock(): MutexGuard<T>` method
2. Add MutexGuard<T> extending Disposable with `get value()` / `set value()` and drop side-effects in onDispose()
3. Update RefCell to add `borrow(): Ref<T>` and `borrow_mut(): RefMut<T>` methods returning Disposable guards
4. Add Ref<T> extending Disposable with read-only `get value()`
5. Add RefMut<T> extending Disposable with read/write `get value()` / `set value()`
6. Existing withMut()/withRef() can remain as convenience wrappers
**Runbook item**: #4 (Update disposable.ts)

### 3.2 PromiseMutex in reactor/index.ts is ad-hoc, not the provided type

**File**: `packages/core/src/reactor/index.ts`
**Lines**: 113-135
**What's wrong**: A local `PromiseMutex` class is defined inline in reactor/index.ts with an `acquire()/release()` pattern. The ownership spec defines `PromiseMutex` with a `run<T>(fn: () => Promise<T>): Promise<T>` API in disposable.ts (or a shared location). The ad-hoc version works but diverges from the spec.
**What the fix is**: Move PromiseMutex to disposable.ts with the `run()` API from provided-types.md. Update reactor/index.ts to use `this.notifyLock.run(async () => { ... })` instead of manual acquire/release.

---

## Priority 4: Async Serialization Gaps

### 4.1 Fire-and-forget async in EntityLiveQuery.create() (init activation)

**File**: `packages/core/src/livequery.ts`
**Lines**: 224-232
**What's wrong**: `void me.activate(1).then(...)` is fire-and-forget. If two LiveQueries are created rapidly for the same collection, their activations can interleave with `evaluateChanges()` calls from `notifyChange()`, because `notifyChange` acquires the `notifyLock` but `activate` does not. This is the same issue flagged in the runbook as "LiveQuery activation race (same bug in Rust, issue #146)".
**What the fix is**: This may be a Rust-parity issue. Document it clearly. Consider whether TS-side serialization via PromiseMutex on the reactor is needed, or if this will be fixed upstream.

### 4.2 Fire-and-forget async in Subscription.fillGapsAndNotify()

**File**: `packages/core/src/reactor/subscription_state.ts`
**Lines**: 530-532
**What's wrong**: `this.fillGapsAndNotify(updateItems, gapsToFill)` is called without await in `evaluateChanges()`. This means gap-fill work (which includes `resultset.write()` -> `rw.done()`) runs outside the `notifyLock` and can interleave with subsequent `notifyChange()` calls. The comment acknowledges this: "Divergence: fire-and-forget async (no task::spawn in JS) [E8]".
**What the fix is**: Per the Rust code, this is intentionally spawned as a separate task. However, in TS, the gap-fill writes to the same resultset that `evaluateChanges` is also writing to, without any serialization. This needs PromiseMutex protection or awaiting within the lock scope.
**Runbook item**: Known issue: "WatcherSet gap-fill has async interleaving risk"

---

## Priority 5: Translation Rule Violations

### 5.1 defineModel() Mutable getters return raw handles, bypassing alive checks

**File**: `packages/core/src/define-model.ts`
**Lines**: 402-416
**What's wrong**: The Mutable getter methods call `entity.getActiveHandle(fieldName, backend)` which returns `{ backend, fieldName, entity }` — a raw object, not a typed LWW<T> or YrsString. This means:
1. The active types returned don't have isWritable() checks
2. Users can call set/insert on a raw handle after the transaction is dead
3. The type system says it returns `LWW<T>` but it actually returns a plain object
**What the fix is**: Mutable getters should construct actual `LWW<T>` / `YrsString` instances (which do have isWritable checks in their set/insert methods). Entity.getActiveHandle() should be removed or refactored to return the real active type.
**Runbook item**: Known issue: "defineModel() returns raw handles — alive checks can be bypassed"

### 5.2 LWW.set() and YrsString.insert() have defensive but weird isWritable checks

**File**: `packages/core/src/property/value/lww.ts`, line 67
**File**: `packages/core/src/property/value/yrs_string.ts`, lines 58, 70, 84, 98
**What's wrong**: The isWritable checks use a guard pattern `if (this.entity && typeof this.entity.isWritable === 'function' && !this.entity.isWritable())`. The `typeof` check is unnecessary and suggests uncertainty about whether entity is properly typed. Since `entity` is typed as `Entity` in the constructor, the check should just be `if (!this.entity.isWritable())`.
**What the fix is**: Simplify to `if (!this.entity.isWritable()) { throw ...; }`. The defensive typeof check suggests the property may sometimes receive non-Entity objects (see 5.1 above). Fix defineModel first, then clean up these checks.

### 5.3 No MIRRORS annotation on test files in packages/core

**Files checked**: Only `packages/core/src/property/backend/lww.test.ts` has a test file; it does have MIRRORS annotation. No other core test files exist in-tree. This is OK — test coverage gaps are a separate concern from annotation compliance.

---

## Priority 6: Style / Consistency

### 6.1 Disposable.dispose() should try/finally onDispose()

**File**: `packages/core/src/disposable.ts`
**Lines**: 84-89
**What's wrong**: Per provided-types.md: "If `onDispose()` throws, the object is still considered disposed and FR is unregistered — the throw propagates to the caller." The current code sets `#disposed = true` and unregisters FR *before* calling `onDispose()`, which is correct. However, if a future onDispose() implementation relies on this ordering, it could be fragile. The spec is satisfied as-is.
**Status**: Compliant. No change needed.

### 6.2 RefCell missing borrow()/borrow_mut() API

**File**: `packages/core/src/disposable.ts`
**Lines**: 193-261
**What's wrong**: The provided-types.md spec says RefCell should have `borrow(): Ref<T>` and `borrow_mut(): RefMut<T>` methods that return Disposable guards. The current implementation only has `withMut()` and `withRef()` closure-based APIs.
**What the fix is**: Add `borrow()` returning `Ref<T>` and `borrow_mut()` returning `RefMut<T>` per the spec. Keep withMut/withRef as convenience wrappers that use borrow/borrow_mut internally.
**Runbook item**: #4

---

## Summary Table

| # | Severity | File | Issue | Runbook |
|---|----------|------|-------|---------|
| 1.1 | **CRITICAL** | transaction.ts | No Disposable, no Symbol.dispose, no auto-rollback | #8 |
| 1.2 | **CRITICAL** | transaction.ts | create/get/edit missing alive checks | #7 |
| 1.3 | **CRITICAL** | resultset.ts | ResultSetWrite uses write()/done() not Disposable | #5 |
| 1.4 | **CRITICAL** | subscription_state.ts | All ResultSetWrite call sites lack try/finally | #5 |
| 2.1 | HIGH | livequery.ts | EntityLiveQuery not Disposable, no assertNotDisposed | #6 |
| 2.2 | HIGH | livequery.ts | LiveQuery<V> not Disposable | #6 |
| 2.3 | HIGH | subscription.ts | ReactorSubscription not Disposable | #6 |
| 2.4 | HIGH | broadcast.ts | ListenerGuard no Disposable/FR | #6 |
| 2.5 | HIGH | subscribe.ts | SubscriptionGuard no Disposable/FR | #6 |
| 2.6 | HIGH | signal/index.ts | ListenerGuard wrapper no Disposable/FR | #6 |
| 3.1 | MEDIUM | disposable.ts | Missing Mutex/MutexGuard/Ref/RefMut types | #4 |
| 3.2 | MEDIUM | reactor/index.ts | Ad-hoc PromiseMutex, not spec-compliant API | #4 |
| 4.1 | MEDIUM | livequery.ts | Fire-and-forget activation race | Known |
| 4.2 | MEDIUM | subscription_state.ts | Fire-and-forget gap-fill interleaving | Known |
| 5.1 | HIGH | define-model.ts | Mutable getters return raw handles, not active types | Known |
| 5.2 | LOW | lww.ts, yrs_string.ts | Defensive typeof checks on entity | Style |
| 6.2 | MEDIUM | disposable.ts | Missing borrow()/borrow_mut() guard API | #4 |

---

## Recommended Fix Order

1. **disposable.ts** (#4) — Add Mutex<T>/MutexGuard<T>, Ref<T>/RefMut<T>, PromiseMutex. Update RefCell. This is the foundation everything else depends on.
2. **transaction.ts** (#7, #8) — Extend Disposable, add alive checks, add Symbol.dispose.
3. **resultset.ts** (#5) — ResultSetWrite extends Disposable; update all call sites in subscription_state.ts to use `using`.
4. **livequery.ts, subscription.ts** (#6) — Wire EntityLiveQuery, LiveQuery, ReactorSubscription to Disposable.
5. **signals package** (#6) — Wire ListenerGuard, SubscriptionGuard to Disposable (needs cross-package Disposable access or a shared package).
6. **define-model.ts** (Known) — Fix Mutable getters to return real LWW/YrsString instances.
7. **Enable eslint-plugin-ankurah** (#14) — Automate future compliance checking.
