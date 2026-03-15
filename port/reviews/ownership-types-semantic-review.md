# Ownership Types — Semantic Review

Reviewer: semantic-reviewer-2
Date: 2026-03-15
Scope: AkObject, Struct, Enum, Drop, Arc, Weak, Borrow, BorrowMut, Mutex, RefCell, AsyncMutex

---

## Status

The current implementation is sound. Earlier drafts of Arc had critical bugs (calling `.drop()` instead of `[disposeSymbol]()`, no per-handle idempotency, no leak registry). These have all been fixed in the current code. The review below covers remaining issues only.

---

## Moderate Issues

### M1. Enum[disposeSymbol]() double-disposes variant value fields

**File**: `packages/base/src/enum.ts:40-48`

The override walks `this.value`'s own properties and disposes them, then calls `super[disposeSymbol]()`. The AkObject cascade in `super[disposeSymbol]()` walks `Object.getOwnPropertyNames(this)`, which includes `type` and `value`. The `value` property is a plain object (no `[disposeSymbol]`), so the AkObject cascade skips it. However, the individual fields inside `this.value` that ARE disposable get disposed by the Enum override first, and then if any of those same objects are also direct properties of `this`, they'd be hit again.

In practice this is harmless — AkObject's idempotency guard (`#dropped` check) prevents double-drop. But the code structurally relies on idempotency for correctness rather than having a clean single-responsibility cascade.

**Severity**: Low. Worth a comment noting the intentional reliance on idempotency.

### M2. Weak.upgrade() constructs Arc via `new (Arc as any)(this.#inner)` — bypasses private constructor

**File**: `packages/base/src/std/arc.ts:82`

Works but is fragile. If Arc's constructor signature changes, this breaks silently at runtime.

### M3. AkObject cascade walks only own string-keyed properties — `#private` fields are invisible

**File**: `packages/base/src/object.ts:23`

`Object.getOwnPropertyNames(this)` does **not** return:
- Private fields (`#foo`) — these are slots, not properties
- Symbol-keyed properties

This means owned disposable values stored in `#private` fields are invisible to cascade. This is **correct and intentional** for the provided types (Borrow, Arc, Mutex, RefCell all use `#private` to hide internals from cascade). But it's a constraint that translators must know: **owned fields that need cascade must be public instance properties**.

**Action**: Document this rule in the translation guidelines.

### M4. ownership.md is stale relative to the actual implementation

**File**: `port/ownership.md`

The spec says:
- `Arc<T>` / `Rc<T>` → `T` (delete wrapper) — **contradicts** the actual `Arc<T>` implementation
- `Weak<T>` → `WeakRef<T>` — **contradicts** the actual `Weak<T>` implementation
- `impl Drop` → `extends Disposable` — **contradicts** the actual `extends Drop` class name

The spec appears to be from an earlier design iteration. It should be updated or marked as superseded.

### M5. Mutex does not extend AkObject — not registered with leak detector

**File**: `packages/base/src/std/sync.ts:20`

`Mutex<T>` is a plain class. If someone creates a Mutex and never uses it, no leak warning. This is probably fine since Mutex itself has no cleanup, but it diverges from the "all ported types extend AkObject" principle. Arc was updated to register with leakRegistry even though it doesn't extend AkObject — Mutex could do the same if desired.

---

## Edge Case Analysis

### E1. Empty structs
An empty `Struct` (no fields) correctly: registers with leak detector in constructor, cascade walks zero properties, `drop()` is a no-op. No issues.

### E2. Unit enum variants
`new MyEnum('UnitVariant', {})` — the `value` is `{}`. The Enum override walks `Object.getOwnPropertyNames({})` which is empty. Correct.

### E3. Nested Arcs — `Arc<Arc<T>>`
Outer Arc's drop decrements outer refcount; when it hits zero, calls `inner[disposeSymbol]()` which calls `innerArc.drop()`, decrementing inner refcount. Correct.

### E4. Weak.upgrade() after all Arcs dropped
Returns `null` when `this.#inner.dropped` is `true`. Correct.

### E5. Struct that owns an Arc that wraps a type with impl Drop
Full cascade chain: `Struct[disposeSymbol]()` → walks fields → `Arc[disposeSymbol]()` → `Arc.drop()` → decrements refcount → if zero, `inner[disposeSymbol]()` → `inner.drop()` (custom) → cascade inner's fields. Correct.

### E6. Borrow fields are correctly skipped by cascade
Cascade finds `this.ref` (a Borrow), calls `ref[disposeSymbol]()` which is a no-op. Borrowed value not dropped. Correct.

### E7. Clone-then-drop-original pattern
`a.clone()` increments refcount. `a.drop()` decrements, `b.drop()` decrements to zero and triggers inner disposal. Correct.

### E8. Arc.clone() after drop
Throws "cannot clone — inner already dropped". Correct.

### E9. Double-drop on same Arc handle
`#released` flag prevents second decrement. Correct.

### E10. Double-drop on same Weak handle
`#released` flag prevents second decrement. Correct.

---

## Composition Integrity Summary

| Scenario | Status | Notes |
|----------|--------|-------|
| AkObject auto-cascade | OK | Walks own string-keyed properties, calls [disposeSymbol] |
| Borrow blocks cascade | OK | No-op [disposeSymbol] |
| Arc refcount lifecycle | OK | Per-handle #released flag, calls [disposeSymbol] on inner |
| Arc leak detection | OK | Registers with leakRegistry in constructor |
| Weak upgrade/drop | OK | #released flag, null-on-dropped |
| Enum variant cascade | OK (minor) | M1: double-dispose masked by idempotency |
| Struct owns Arc owns Drop | OK | Full cascade chain verified |
| Drop subclass cleanup | OK | Abstract drop() called before cascade |
| Mutex/RefCell guards | OK | Guards use #private fields, don't cascade to guarded value |

---

## Real-Usage Test Review

Reviewed `packages/base/__tests__/real_usage.test.ts` — 22 tests, all passing.

### Mock Fidelity Issues

1. **EntityKind.Transacted `trxAlive`**: Rust has `Arc<AtomicBool>`, test uses `{ value: boolean }`. Should use `Arc.new({ value: boolean })` to model the shared-ownership aspect (multiple snapshots share one liveness flag).

2. **NodeMessage missing `UnsubscribeEntities` variant**: Rust has 6 variants, test has 5.

3. **NodeRequest** in Rust has `body: NodeRequestBody` (an enum). Test mock only has `id/to/from`.

4. **NodeResponse** in Rust has `request_id/from/to/body`. Test mock only has `requestId`.

5. **CausalAssertionFragment** in Rust has `relation: CausalRelation` (an enum) + `attestations: AttestationSet`. Test mock has `relation: string`.

### Missing Ownership Scenarios

1. **Multi-owner Entity via Arc**: Test "Transacted variant owns upstream entity" creates a sole-owner Entity. Should also test: dispose EntityKind::Transacted when another Entity clone exists — inner must survive.

2. **Entity.snapshot()** pattern: Creating a transacted fork sharing `trx_alive`, killing the transaction, verifying `is_writable()` returns false. Core Rust pattern, not tested.

3. **Nested ownership: LiveQuery → ReactorSubscription**: `Inner` in livequery.rs holds `subscription: ReactorSubscription`. When LiveQuery's inner drops, it runs `impl Drop` and cascades to the subscription. No test for this multi-layer chain.

4. **Borrow in a real struct**: No test shows a struct with a `Borrow<T>` field surviving cascade (parent drops, borrowed value lives).

5. **WeakEntity pattern**: Weak→upgrade→None-after-drop with Entity shapes (not just raw Arc).

### Tests Verified Correct

- ReactorSubscription single/multi-owner drop, using block, cascade to broadcast — all correctly exercise the Arc→Drop cascade chain.
- Entity clone, mutex access, cascade — correct.
- Enum match/is/cascade for EntityKind, DeltaContent, NodeMessage — correct.
- Proto structs (ProtoEvent, Clock) — correct plain-data cascade.
