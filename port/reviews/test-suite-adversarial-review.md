# Test Suite — Adversarial Review

**Reviewer**: adversarial-reviewer-2
**Date**: 2026-03-15
**Files reviewed**:
- `packages/base/__tests__/ownership.test.ts` (28 tests)
- `packages/base/__tests__/real_usage.test.ts` (24 tests)
**Reference**: Rust source at `/Users/daniel/ak/ankurah/core/src/`

---

## 1. Missing Edge Cases

### 1a. CRITICAL: Arc.drop() calls .drop() not [disposeSymbol]() — NOT TESTED

The previous adversarial review (Finding 11 in `ownership-types-adversarial-review.md`) identified that `Arc.drop()` at arc.ts:82-86 calls `value.drop()` instead of `value[disposeSymbol]()`. This means the auto-cascade is skipped for the inner value.

**ownership.test.ts line 141-148** has a test titled "inner cascade works through Arc":
```typescript
test('inner cascade works through Arc', () => {
  const owner = new Owner();
  const a = Arc.new(owner);
  a.drop();
  expect(owner.inner.dropCount).toBe(1);
});
```

**This test PASSES, but only because of a coincidence.** Let me trace it:

1. `a.drop()` → strongCount hits 0 → calls `owner.drop()` (NOT `owner[disposeSymbol]()`).
2. `Owner.drop()` at line 19 is: `drop(): void { /* custom cleanup */ }` — a no-op.
3. But the test expects `owner.inner.dropCount` to be 1. How?

Wait — `owner.inner` is an `Inner` instance. `Inner.drop()` increments `dropCount`. But who calls `Inner.drop()`?

Actually re-reading: `Owner.drop()` is a no-op, and Arc calls `owner.drop()` not `owner[disposeSymbol]()`. So `owner.inner.drop()` is NEVER called. `owner.inner.dropCount` should be 0.

**If this test actually passes, it would mean my analysis is wrong.** Let me re-check... No, the Arc code at arc.ts:82-86 explicitly calls `(val as any).drop()`. For `Owner`, that's the no-op `drop()`. The cascade that would reach `inner` lives in `[disposeSymbol]()` which is NOT called.

**Prediction: This test SHOULD FAIL if run.** If it passes, something else is going on (perhaps the test infrastructure or imports differ from what I read). This needs to be verified by actually running it.

**If the test does pass**: Then either (a) the Arc code I read is stale, or (b) there's an import indirection I'm missing. Either way, running `bun test packages/base/__tests__/ownership.test.ts` is essential.

**If the test does fail**: The critical Arc.drop() bug is confirmed and this test was never actually run, or was run against different code.

### 1b. No test for Arc wrapping a non-Drop AkObject

All Arc tests use `Inner extends Drop`. There's no test for:
```typescript
class PlainStruct extends Struct { child: SomeDisposable; }
const arc = Arc.new(new PlainStruct());
arc.drop(); // Does PlainStruct's cascade run?
```

Since `PlainStruct` inherits `drop()` as a no-op from `AkObject`, and `Arc.drop()` calls `.drop()` (not `[disposeSymbol]()`), the cascade would be skipped. This is a distinct scenario from the Drop subclass case.

### 1c. No test for BorrowMut

`BorrowMut` has zero test coverage. Only `Borrow` is tested. While `BorrowMut` is structurally similar, it has a setter (`set value(v)`) that Borrow lacks. Tests needed:
- `BorrowMut[disposeSymbol]()` is a no-op
- `BorrowMut.value` getter works
- `BorrowMut.value` setter works
- Parent cascade skips BorrowMut

### 1d. No test for Enum with Arc in variant data

The `real_usage.test.ts` tests `EntityKind.Transacted` which has an `Entity` (struct) in the variant, but no test has an `Arc<T>` directly as a variant field value. The interaction between Enum's dispose (cascades value fields) and Arc's dispose is untested at the variant level.

### 1e. No test for Weak.upgrade() returning a usable Arc that participates in refcounting

The Weak tests check that `upgrade()` returns non-null and that `strongCount` increments. But they don't test the full lifecycle:
```typescript
const a = Arc.new(inner);
const w = a.downgrade();
a.drop(); // strongCount 0 if no upgrade
// But if we upgrade BEFORE dropping:
const upgraded = w.upgrade(); // strongCount should be 2
a.drop(); // strongCount 1
// inner should NOT be dropped yet
upgraded.drop(); // strongCount 0, NOW inner drops
```

This "upgrade keeps alive" pattern is exactly what `WeakEntityLiveQuery` does in Rust.

### 1f. No test for RefCell.borrow() after mutable borrow is released

There IS a test at line 287-293, but it only checks `borrow()` after `borrow_mut()` releases. There's no test for:
- `borrow_mut()` after `borrow()` releases
- Multiple sequential `borrow_mut()` cycles
- `borrow_mut()` inside a `borrow()` scope (should throw)

### 1g. No test for MutexGuard.value setter

`MutexGuard` has `set value(v)` but no test exercises it. Only `guard.value.x = 2` (mutating the inner object) is tested, not `guard.value = newObject` (replacing the entire value).

### 1h. No test for assertNotDropped on Ref/RefMut after dispose

`Ref` and `RefMut` extend `Drop` which extends `AkObject`. After dispose, `ref.value` should throw via `assertNotDropped()`. Not tested.

---

## 2. Tests That Pass for the Wrong Reason

### 2a. "inner cascade works through Arc" (ownership.test.ts:141-148)

As analyzed in 1a above, this test claims to verify that Arc cascade reaches nested fields. If Arc.drop() calls `.drop()` instead of `[disposeSymbol]()`, the cascade doesn't run, and `owner.inner.dropCount` should be 0, not 1. **This test either fails (confirming the bug) or passes for a reason I can't see from static analysis.**

### 2b. "cascade drops inner state and broadcast" (real_usage.test.ts:234-241)

```typescript
test('cascade drops inner state and broadcast', () => {
  const e = Entity.create('e-3', 'albums');
  const inner = e.inner.value;
  const broadcast = inner.broadcast;
  e[disposeSymbol]();
  expect(inner.isDropped).toBe(true);
  expect(broadcast.isDropped).toBe(true);
});
```

The cascade path is: `e[disposeSymbol]()` → cascades to `e.inner` (Arc) → `arc[disposeSymbol]()` → `arc.drop()` → calls `inner.drop()` (NOT `inner[disposeSymbol]()`).

`EntityInner` extends `Struct` extends `AkObject`. Its `drop()` is the inherited no-op. So `inner.drop()` does nothing. The cascade that would set `inner.isDropped = true` and reach `broadcast` lives in `inner[disposeSymbol]()`, which is never called.

**Prediction: `inner.isDropped` should be `false` and `broadcast.isDropped` should be `false`.** This test should fail.

**Same root cause as 2a — the Arc.drop() bug.**

### 2c. "inner broadcast is cascade-dropped" (real_usage.test.ts:200-206)

Same analysis. `sub[disposeSymbol]()` cascades to `sub.inner` (Arc), Arc.drop() calls `ReactorSubInner.drop()` (the custom one that pushes to `unsubscribedIds`). But the **cascade** from `ReactorSubInner` to its `broadcast` field runs via `[disposeSymbol]()`, not `.drop()`.

So `ReactorSubInner.drop()` fires (unsubscribe works), but `broadcast.isDropped` should remain `false`.

**Prediction: This test should fail on the `broadcast.isDropped` assertion.**

### 2d. "Transacted variant owns upstream entity" cascade check (real_usage.test.ts:255-272)

```typescript
const innerRef = upstream.inner.value;
kind[disposeSymbol]();
expect(innerRef.isDropped).toBe(true);
```

Path: `kind[disposeSymbol]()` → Enum cascade → disposes `upstream` (Entity) field in value → `upstream[disposeSymbol]()` → cascades to `upstream.inner` (Arc) → `arc.drop()` → calls `entityInner.drop()` (no-op, EntityInner doesn't impl Drop). `entityInner[disposeSymbol]()` is NOT called.

**Prediction: `innerRef.isDropped` should be `false`.** Test should fail.

### Summary of 2a-2d

All four tests that check cascade behavior **through** an Arc boundary depend on `Arc.drop()` calling `[disposeSymbol]()` on the inner. If it calls `.drop()` as the code reads, all four should fail. **Either the code I read is not what's running, or these tests have never been executed.**

---

## 3. Scenarios from Rust Not Covered

### 3a. Transaction Drop pattern

Rust has `impl Drop for Transaction` that stores `false` to `self.alive` (an `Arc<AtomicBool>`). This is a critical safety mechanism — entities created in a transaction check `trx_alive` to know if their transaction was committed or dropped. No test models this pattern.

Suggested test:
```typescript
class Transaction extends Drop {
  alive: { value: boolean };
  constructor(alive: { value: boolean }) { super(); this.alive = alive; }
  drop(): void { this.alive.value = false; }
}
// Entity checks trx_alive to know if transaction is still valid
```

### 3b. WeakEntityLiveQuery upgrade-and-use pattern

Rust pattern at `livequery.rs:291-295`:
```rust
if let Some(livequery) = self.upgrade() {
    livequery.activate(version).await;
}
```

This is a Weak that upgrades, does work, then the upgraded Arc goes out of scope (drop). No test covers this "upgrade, use, auto-drop" lifecycle.

### 3c. Weak in a loop — upgrade-and-break pattern

Rust at `node.rs:202-206`:
```rust
loop {
    tokio::time::sleep(...).await;
    let Some(node) = weak_node.upgrade() else { break; };
    // use node
}
```

Pattern: repeatedly upgrade a Weak, break when it returns None. Tests should verify that each `upgrade()` correctly increments/decrements refcount per iteration without leaking.

### 3d. EntityKind::Transacted holding both Arc<AtomicBool> and Entity

The Rust `EntityKind::Transacted` variant holds `trx_alive: Arc<AtomicBool>` AND `upstream: Entity`. The test at real_usage.test.ts:119-128 models `trxAlive` as `{ value: boolean }` (a plain object, not Arc). This misses the Arc-in-enum-variant drop behavior.

### 3e. Clone of ReactorSubscription used across async boundaries

In Rust, `ReactorSubscription` is cloned and sent to spawned tasks. The test at real_usage.test.ts:178-189 tests clone + sequential drop, but not the pattern where cloned handles are used concurrently and dropped at different times.

### 3f. RefCell borrow guard escaping its scope

Rust enforces this at compile time. In TS, nothing prevents:
```typescript
let escaped: Ref<T>;
{ using r = cell.borrow(); escaped = r; }
// escaped is now a dangling guard — but in TS, is it still usable?
```

After `using` block exits, `r[disposeSymbol]()` fires, calling `#release()`. The RefCell thinks the borrow is released. But `escaped` still holds a reference to the Ref. `escaped.value` would call `assertNotDropped()` and throw — but this isn't tested.

---

## 4. Tests That Would Pass If Base Types Were Broken

### 4a. Tests that only check construction, not disposal

- "entity clone shares inner" (real_usage.test.ts:210-217) — checks `strongCount` and `id()`, but the final `e1[disposeSymbol]()` / `e2[disposeSymbol]()` have no assertions after them. If disposal was completely broken (no-op), this test still passes.

- "Primary variant" (real_usage.test.ts:245-253) — matches and checks return value. The `kind[disposeSymbol]()` at the end has no post-assertion. Would pass with broken dispose.

- "is() type narrowing" tests — purely about match/is logic, don't exercise ownership at all. Would pass with stub types.

### 4b. Tests with no negative assertions

- "construction and methods" (real_usage.test.ts:395-402) — never calls dispose. Would pass if Struct was just a plain object with no ownership semantics.

- Several "match" and "is()" tests are pure data tests that don't exercise ownership machinery at all.

### 4c. Idempotent dispose tests don't verify the FIRST dispose did anything

- "dispose is idempotent" (ownership.test.ts:51-56) — checks `dropCount` is 1 after two disposes. But if the first dispose was also broken (dropCount stays 0), and then the second incremented to 1, you'd still get 1. This is unlikely but the test structure doesn't distinguish "first worked, second was no-op" from "first was no-op, second worked."

Actually, looking more carefully: the idempotency guard (`#dropped = true` on first call) means the second call returns immediately. So if dropCount is 1, the first call must have done it. This is fine.

---

## 5. The `#` Private Field Gap — Is It Tested?

**No.** There is no test that:

1. Creates a Struct/AkObject subclass with a `#` private field holding a disposable
2. Disposes the parent
3. Asserts the `#` field was NOT disposed (documenting the known limitation)

This is important to test because:
- It documents the invariant ("don't use # for owned disposables")
- It catches regressions if someone changes the cascade to use `Reflect.ownKeys()` or similar
- It serves as a canary — if this test starts passing, the cascade mechanism changed

**Recommended test:**
```typescript
test('# private fields are NOT reached by auto-cascade (known limitation)', () => {
  class HasPrivate extends Struct {
    #secret: Inner;
    constructor() { super(); this.#secret = new Inner(); }
    getSecret(): Inner { return this.#secret; }
  }
  const h = new HasPrivate();
  const secret = h.getSecret();
  h[disposeSymbol]();
  // # fields are invisible to Object.getOwnPropertyNames() — cascade does NOT reach them
  expect(secret.dropCount).toBe(0); // documents the limitation
});
```

---

## Summary

| Category | Count | Severity |
|----------|-------|----------|
| Missing edge cases | 8 | HIGH (1a, 1b), MEDIUM (rest) |
| Tests passing for wrong reason | 4 | CRITICAL — all depend on Arc.drop() bug |
| Rust patterns not covered | 6 | MEDIUM-HIGH |
| Tests that pass with broken types | 5+ | LOW (informational) |
| `#` private field gap tested? | No | MEDIUM — should be a documented test |

### Top Priority

**Run the tests.** The static analysis predicts that at least 4 tests (2a-2d) should FAIL due to the Arc.drop() bug (calls `.drop()` not `[disposeSymbol]()`). If they pass, either the code I read is stale or there's runtime behavior I'm not seeing. Either way, the answer determines whether this is a 4-bug report or a "my analysis was wrong" report.
