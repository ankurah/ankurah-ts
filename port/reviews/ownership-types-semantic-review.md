# Ownership Types — Semantic Review

Reviewer: semantic-reviewer-2
Date: 2026-03-15
Scope: AkObject, Struct, Enum, Drop, Arc, Weak, Borrow, BorrowMut, Mutex, RefCell, AsyncMutex

---

## Critical Issues

### C1. Arc.drop() calls value.drop() but NOT value[Symbol.dispose]() — cascade is skipped

**File**: `packages/base/src/std/arc.ts:82-86`

When the last Arc drops, the code does:
```ts
if (val && typeof (val as any).drop === 'function') {
  (val as any).drop();
}
```

This calls the custom `drop()` method but **does not call `[disposeSymbol]()`**. That means the auto-cascade walk in `AkObject[disposeSymbol]()` never runs. If the inner value is a `Drop` subclass that owns other disposable fields, those fields are **never cleaned up**.

**Concrete scenario**: `ReactorSubInner` has `impl Drop` and owns a `Broadcast<ReactorUpdate>`. In Rust, dropping ReactorSubInner runs its `drop()` impl, then Rust's compiler-generated drop glue cascades to all fields. In the TS port, `Arc.drop()` would call `inner.drop()` but never trigger the cascade to owned fields like the broadcast.

**Fix**: Call `val[disposeSymbol]()` instead of `val.drop()`. The `[disposeSymbol]()` method in AkObject already calls `this.drop()` first and then cascades — that's exactly the right sequence.

```ts
if (val && typeof (val as any)[disposeSymbol] === 'function') {
  (val as any)[disposeSymbol]();
}
```

### C2. Enum[disposeSymbol]() double-disposes variant value fields

**File**: `packages/base/src/enum.ts:40-48`

The override walks `this.value`'s own properties and disposes them, then calls `super[disposeSymbol]()`. But `super[disposeSymbol]()` (in AkObject) walks `Object.getOwnPropertyNames(this)`, which includes `type` and `value`. The `value` object itself doesn't have `[disposeSymbol]` (it's a plain object), so it's harmless in that direction — **but** the AkObject cascade also walks all own properties, and `type`/`value` are set in the constructor as instance properties.

The actual problem is subtler: if `this.value` is itself an AkObject (e.g., an enum variant that holds a single struct), the Enum override disposes it, and then the AkObject cascade in `super[disposeSymbol]()` disposes it **again**. The idempotency guard in AkObject protects against double-drop of AkObjects, so this is not a correctness bug per se, but it's a semantic smell — the code accidentally relies on idempotency for correctness.

**Severity**: Low (idempotency saves it), but worth a comment or restructuring the override to delegate entirely to super.

### C3. Arc does not extend AkObject — invisible to parent cascade and leak detector

**File**: `packages/base/src/std/arc.ts:36`

`Arc<T>` is a plain class, not an `AkObject`. This means:
1. **No leak detection**: Arc instances are not registered with the `FinalizationRegistry`. If someone creates an Arc and forgets to drop it, no warning is emitted.
2. **Not visible to parent cascade**: If an AkObject has an `Arc<T>` field, the parent's `[disposeSymbol]()` cascade sees it has `[disposeSymbol]` and calls it — that part works. But the Arc itself doesn't participate in the leak registry.

This is a design choice, not necessarily a bug — but it diverges from the principle that "all ported Rust types inherit AkObject." An `Arc` is a ported Rust type.

**Risk**: Leaked Arcs holding Drop-implementing inners will never warn. The inner's FinalizationRegistry entry won't fire either because the Arc holds a strong JS reference to it.

### C4. Arc.drop() is not idempotent per-handle — calling drop() twice on the same Arc decrements twice

**File**: `packages/base/src/std/arc.ts:77-87`

```ts
drop(): void {
    if (this.#inner.strongCount <= 0) return; // already fully released
    this.#inner.strongCount--;
    ...
}
```

The guard checks if `strongCount <= 0`, but there's no per-instance `#dropped` flag. If you call `arc.drop()` twice on the same Arc handle:
- First call: strongCount goes from N to N-1. Fine.
- Second call: strongCount goes from N-1 to N-2. **Bug** — this Arc handle was already dropped, it shouldn't decrement again.

In Rust this can't happen because drop takes ownership (`self`, not `&self`). In JS, nothing prevents calling `.drop()` twice on the same reference.

**Fix**: Add a per-instance `#dropped = false` flag and bail early if already dropped.

```ts
#dropped = false;

drop(): void {
    if (this.#dropped) return;
    this.#dropped = true;
    this.#inner.strongCount--;
    if (this.#inner.strongCount === 0) {
        this.#inner.dropped = true;
        const val = this.#inner.value;
        if (val && typeof (val as any)[disposeSymbol] === 'function') {
            (val as any)[disposeSymbol]();
        }
    }
}
```

Similarly, `[disposeSymbol]()` delegates to `drop()` so this fix covers both paths.

---

## Moderate Issues

### M1. Weak.upgrade() constructs Arc via `new (Arc as any)(this.#inner)` — bypasses private constructor intent

**File**: `packages/base/src/std/arc.ts:138-139`

This works but is fragile. If Arc's constructor signature changes, this breaks silently at runtime. Consider adding a `static fromInner<T>(inner: ArcInner<T>): Arc<T>` private/internal factory method.

### M2. Weak has no protection against double-drop

**File**: `packages/base/src/std/arc.ts:145-149`

Same pattern as C4 — calling `weak.drop()` twice decrements `weakCount` twice. Needs a per-instance `#dropped` flag.

### M3. AkObject cascade walks only own *enumerable* property names — private fields (#field) are invisible

**File**: `packages/base/src/object.ts:23`

`Object.getOwnPropertyNames(this)` returns string-keyed own properties. It does **not** return:
- Private fields (`#foo`) — these are not properties at all in JS, they're slots
- Symbol-keyed properties

This means if a ported struct stores an owned disposable value in a `#private` field, cascade won't reach it. This is actually **correct behavior** for the current code since `Borrow`, `Arc`, etc. use `#private` fields precisely to hide their internals. But it means the translation rule must be: "owned fields that need cascade must be public or at least non-private instance properties."

**Action**: Document this constraint clearly. It's not a bug, but it's a footgun for translators.

### M4. MutexGuard, Ref, RefMut extend Drop (which extends AkObject) — cascade walks their fields

**File**: `packages/base/src/std/sync.ts:50`, `packages/base/src/std/cell.ts:98,125`

These guards store their values in `#private` fields, so cascade can't reach them — which is correct since guards don't own the guarded value. But if they stored values as public fields, the cascade would try to dispose the guarded value on guard drop. The current implementation is safe because of the `#private` field pattern, but this is an implicit invariant worth documenting.

### M5. ownership.md is stale relative to the actual implementation

**File**: `port/ownership.md`

The spec says:
- `Arc<T>` / `Rc<T>` → `T` (delete wrapper) — **contradicts** the actual `Arc<T>` implementation
- `Weak<T>` → `WeakRef<T>` — **contradicts** the actual `Weak<T>` implementation
- `impl Drop` → `extends Disposable` — **contradicts** the actual `extends Drop` class name

The spec appears to be from an earlier design iteration before the decision to provide 1:1 ownership types. It should be updated to reflect the current implementation, or clearly marked as superseded.

---

## Edge Case Analysis

### E1. Empty structs
An empty `Struct` (no fields) correctly: registers with leak detector in constructor, cascade walks zero properties, `drop()` is a no-op. No issues.

### E2. Unit enum variants
`new MyEnum('UnitVariant', {})` — the `value` is `{}`. The Enum override walks `Object.getOwnPropertyNames({})` which is empty. Correct.

### E3. Nested Arcs — `Arc<Arc<T>>`
Works correctly in principle: outer Arc's drop decrements outer refcount; when it hits zero, it drops the inner Arc (which decrements inner refcount). However, combined with C1, the outer Arc's drop would call `innerArc.drop()` directly, which is actually fine for Arc since Arc.drop() is the intended method. But if the fix for C1 changes to call `[disposeSymbol]()`, that also works since Arc's `[disposeSymbol]()` delegates to `drop()`.

### E4. Weak.upgrade() after all Arcs dropped
```ts
const a = Arc.new(value);
const w = a.downgrade();
a.drop();
const upgraded = w.upgrade(); // returns null ✓
```
Correct — `this.#inner.dropped` is `true`, returns `null`.

### E5. Struct that owns an Arc that wraps a type with impl Drop
```ts
class MyStruct extends Struct {
    inner: Arc<MyDropType>;
    constructor(inner: Arc<MyDropType>) {
        super();
        this.inner = inner;
    }
}
```
When MyStruct is disposed:
1. AkObject cascade finds `this.inner` (an Arc), calls `inner[disposeSymbol]()`
2. Arc's `[disposeSymbol]()` calls `this.drop()`, which decrements refcount
3. If refcount hits zero, calls `value.drop()` (C1 bug — should call `[disposeSymbol]()`)

With C1 fixed, this chain works correctly.

### E6. Borrow fields are correctly skipped by cascade
```ts
class MyStruct extends Struct {
    ref: Borrow<SomeDropType>;
}
```
Cascade finds `this.ref`, calls `ref[disposeSymbol]()` which is a no-op. The borrowed value is not dropped. Correct.

### E7. Clone-then-drop-original pattern
```ts
const a = Arc.new(myDropValue);
const b = a.clone(); // strongCount = 2
a[disposeSymbol]();  // strongCount = 1, inner NOT dropped
b[disposeSymbol]();  // strongCount = 0, inner dropped ✓
```
Correct.

### E8. Arc.clone() after drop
```ts
const a = Arc.new(value);
a.drop();            // strongCount = 0, inner dropped
a.clone();           // throws "cannot clone — inner value has been dropped" ✓
```
Correct.

---

## Composition Integrity Summary

| Scenario | Status | Notes |
|----------|--------|-------|
| AkObject auto-cascade | OK | Walks own properties, calls [disposeSymbol] |
| Borrow blocks cascade | OK | No-op [disposeSymbol] |
| Arc refcount lifecycle | BUG | C1: calls drop() not [disposeSymbol](); C4: no per-handle idempotency |
| Weak upgrade/drop | OK (minor) | M2: no double-drop guard |
| Enum variant cascade | OK (minor) | C2: double-dispose masked by idempotency |
| Struct owns Arc owns Drop | BUG | Cascade works but inner's sub-fields lost (C1) |
| Drop subclass cleanup | OK | Abstract drop() called before cascade |
| Mutex/RefCell guards | OK | Guards don't cascade to guarded value (correct) |
| Leak detection | GAP | C3: Arc not registered; leaked Arc = silent |

---

## Recommended Fix Priority

1. **C1** — Arc inner disposal must call `[disposeSymbol]()` not `.drop()` — correctness bug, fields will leak
2. **C4** — Arc per-handle idempotency flag — correctness bug, double-drop corrupts refcount
3. **M5** — Update ownership.md to match implementation — spec drift causes translator confusion
4. **C3** — Register Arc with leak detector — observability gap
5. **M2** — Weak double-drop guard — minor correctness
6. **C2** — Enum cascade restructure — code clarity
