# Ownership Types — Adversarial Red-Team Review

**Reviewer**: adversarial-reviewer-2
**Date**: 2026-03-15
**Files reviewed**:
- `packages/base/src/object.ts` (AkObject — base class, auto-cascade)
- `packages/base/src/struct.ts` (Struct — trivial subclass of AkObject)
- `packages/base/src/enum.ts` (Enum — variant-aware dispose)
- `packages/base/src/std/arc.ts` (Arc<T>, Weak<T>)
- `packages/base/src/std/borrow.ts` (Borrow<T>, BorrowMut<T>)
- `packages/base/src/std/drop.ts` (Drop, DropGuard)
- `packages/base/src/std/cell.ts` (RefCell<T>, Ref<T>, RefMut<T>)
- `packages/base/src/std/sync.ts` (Mutex<T>, MutexGuard<T>)
- `packages/base/src/drop_registry.ts` (disposeSymbol, leakRegistry)

---

## Scenario 1: Struct owns Arc<Inner> and Borrow<Other>. Drop the struct.

### Setup

```typescript
class Inner extends Drop {
  drop() { console.log('Inner.drop()'); }
}

class Other extends AkObject {}

class MyStruct extends Struct {
  arcField: Arc<Inner>;
  borrowField: Borrow<Other>;
  constructor(arc: Arc<Inner>, borrow: Borrow<Other>) {
    super();
    this.arcField = arc;
    this.borrowField = borrow;
  }
}

const other = new Other();
const inner = new Inner();
const arc = Arc.new(inner);
const s = new MyStruct(arc, new Borrow(other));
s[Symbol.dispose]();
```

### Trace

1. `s[disposeSymbol]()` enters `AkObject.[disposeSymbol]()` (object.ts:18).
2. `#dropped` is false → sets `#dropped = true` (line 20).
3. Calls `this.drop()` → `AkObject.drop()` is a no-op (Struct doesn't override it). **OK.**
4. Iterates `Object.getOwnPropertyNames(this)`. This yields `["arcField", "borrowField"]`.
   - **Note**: `#dropped` is a private field — `getOwnPropertyNames` does NOT enumerate it. **Correct.**
5. `this.arcField` is the Arc instance. `typeof arc[disposeSymbol]` → `'function'`. Calls `arc[disposeSymbol]()`.
   - `Arc.[disposeSymbol]()` (arc.ts:107) delegates to `Arc.drop()` (arc.ts:77).
   - `strongCount` is 1 → decrements to 0 → sets `dropped = true` → calls `inner.drop()` → logs "Inner.drop()". **CORRECT.**
6. `this.borrowField` is the Borrow instance. `typeof borrow[disposeSymbol]` → `'function'`. Calls `borrow[disposeSymbol]()`.
   - `Borrow.[disposeSymbol]()` (borrow.ts:23) is a no-op. **CORRECT — does not propagate.**

### Verdict: PASS

Arc decrements correctly. Borrow does NOT propagate. The auto-cascade correctly distinguishes the two.

---

## Scenario 2: Enum variant holds Arc<T>. Match, extract, clone. Drop enum.

### Setup

```typescript
class Payload extends Drop {
  drop() { console.log('Payload.drop()'); }
}

type MyEnumV = {
  HasArc: { data: Arc<Payload> };
  Empty: {};
};

class MyEnum extends Enum<MyEnumV> {
  static HasArc(data: Arc<Payload>) { return new MyEnum('HasArc', { data }); }
  static Empty() { return new MyEnum('Empty', {}); }
}

const payload = new Payload();
const arc = Arc.new(payload);
const e = MyEnum.HasArc(arc);

// Match and extract the Arc, clone it
let extracted: Arc<Payload> | null = null;
e.match({
  HasArc: (v) => { extracted = v.data.clone(); },
  Empty: () => {},
});

// Now drop the enum
e[Symbol.dispose]();
```

### Trace

1. `e[disposeSymbol]()` enters `Enum.[disposeSymbol]()` (enum.ts:40).
2. `this.isDropped` is false → proceeds.
3. Iterates `Object.getOwnPropertyNames(this.value)`. `this.value` is `{ data: arc }`. Yields `["data"]`.
4. `field = this.value.data` → the Arc instance (the original, not the clone).
5. `typeof field[disposeSymbol]` → `'function'`. Calls `arc[disposeSymbol]()` → `arc.drop()`.
   - `strongCount` was 2 (original + clone). Decrements to 1. Inner NOT dropped. **CORRECT.**
6. Then calls `super[disposeSymbol]()` → `AkObject.[disposeSymbol]()`.
   - Iterates own properties of the enum: `["type", "value"]`.
   - `this.type` is `"HasArc"` (string). No `disposeSymbol`. **Skipped.**
   - `this.value` is `{ data: arc }`. Does this plain object have `[disposeSymbol]`? **NO.** Plain objects don't. Skipped.

### BUG FOUND: Double-dispose of Arc fields

Wait — re-read step 5. The Enum override disposes `this.value.data` (the Arc). Then in step 6, `super[disposeSymbol]()` iterates the Enum's own properties and finds `this.value`. Since `this.value` is a plain object (no `[disposeSymbol]`), it's skipped — so the Arc inside it is **not** disposed a second time.

**Actually no bug here** — the plain `value` object doesn't have `[disposeSymbol]`, so the parent cascade skips it. The Arc was only disposed once (in the Enum override). **CORRECT.**

7. Later: `extracted!.drop()` → strongCount goes from 1 to 0 → `payload.drop()` fires. **CORRECT.**

### Verdict: PASS

Arc refcount works correctly through enum match/extract/clone/drop.

---

## Scenario 3: Arc → downgrade → drop all strong refs → Weak.upgrade()

### Setup

```typescript
const val = { name: 'test' };
const arc1 = Arc.new(val);
const arc2 = arc1.clone(); // strongCount = 2
const weak = arc1.downgrade(); // weakCount = 1

arc1.drop(); // strongCount = 1
arc2.drop(); // strongCount = 0, inner.dropped = true

const upgraded = weak.upgrade();
```

### Trace

1. `Arc.new(val)` → inner = `{ value: val, strongCount: 1, weakCount: 0, dropped: false }`.
2. `arc1.clone()` → strongCount = 2. `arc2` shares same `#inner`.
3. `arc1.downgrade()` → weakCount = 1. `weak` shares same `#inner`.
4. `arc1.drop()` → strongCount = 1. Not zero, no cleanup.
5. `arc2.drop()` → strongCount = 0. `dropped = true`. `val` has no `.drop()` method, so no drop call. **OK.**
6. `weak.upgrade()` (arc.ts:134): checks `this.#inner.dropped` → true. Returns `null`. **CORRECT.**

### Verdict: PASS

Weak correctly returns null when all strong refs are dropped.

---

## Scenario 4: Nested ownership: Struct A owns Struct B which owns Arc<C> where C has impl Drop. Drop A.

### Setup

```typescript
class C extends Drop {
  drop() { console.log('C.drop()'); }
}

class B extends Struct {
  arcC: Arc<C>;
  constructor(arc: Arc<C>) { super(); this.arcC = arc; }
}

class A extends Struct {
  b: B;
  constructor(b: B) { super(); this.b = b; }
}

const c = new C();
const arc = Arc.new(c);
const b = new B(arc);
const a = new A(b);

a[Symbol.dispose]();
```

### Trace

1. `a[disposeSymbol]()` → AkObject cascade.
2. `a.drop()` → no-op (Struct base).
3. Iterates `Object.getOwnPropertyNames(a)` → `["b"]`.
4. `a.b` is a `B` (extends Struct extends AkObject). `typeof b[disposeSymbol]` → `'function'`. Calls `b[disposeSymbol]()`.
5. Inside `b[disposeSymbol]()`:
   - `b.drop()` → no-op.
   - Iterates `Object.getOwnPropertyNames(b)` → `["arcC"]`.
   - `b.arcC` is Arc<C>. Calls `arc[disposeSymbol]()` → `arc.drop()`.
   - `strongCount` 1 → 0. `dropped = true`. `c.drop()` fires → logs "C.drop()". **CORRECT.**

### Verdict: PASS

Nested ownership cascades correctly. C's custom drop fires when A is disposed at the top level.

---

## Scenario 5: Auto-cascade uses Object.getOwnPropertyNames(). Does it miss private fields (#field)? Inherited fields?

### 5a: Private fields (#field)

```typescript
class HasPrivate extends Struct {
  #secret: Arc<SomeDroppable>;
  constructor(arc: Arc<SomeDroppable>) { super(); this.#secret = arc; }
}
```

`Object.getOwnPropertyNames(this)` does **NOT** include `#secret`. ECMAScript private fields (those with `#` prefix) are not string-keyed properties — they're stored in a separate internal slot. `getOwnPropertyNames` only returns string-keyed own properties.

**BUG: Private fields with `#` are invisible to the auto-cascade. They will NOT be disposed.**

The Arc stored in `#secret` will never have its refcount decremented by the cascade. If it's the last strong reference, the inner value leaks.

**Severity**: HIGH — any subclass of AkObject/Struct/Enum that uses `#field` to store an owned disposable will leak.

**Mitigation in current code**: Looking at the actual implementations — `AkObject.#dropped` is a boolean (not disposable), so the cascade correctly ignoring it is fine. `Arc.#inner` is the shared inner state (not disposable). `Borrow.#value` being invisible is fine (Borrow is non-owning). `BorrowMut.#value` same. `RefCell` uses `#value`, `#state`, `#onMutRelease`, `#label` — but RefCell doesn't extend AkObject, so no cascade applies. `Drop.#dropped` (in DropGuard) is a boolean.

**Current code is safe**, but only because the existing types happen to not store disposables in `#` fields. This is an **implicit invariant** that is not documented or enforced. A future developer who writes `#field: Arc<T>` in a Struct subclass will silently leak.

**Recommendation**: Document this as a rule: "Never store owned disposable values in `#` private fields of AkObject subclasses. Use TypeScript `private` (which compiles to regular properties) instead of ECMAScript `#` private fields. Alternatively, override `drop()` to manually dispose `#` fields."

### 5b: Inherited fields

```typescript
class Parent extends Struct {
  ownedA: Arc<Foo>;
  constructor(a: Arc<Foo>) { super(); this.ownedA = a; }
}

class Child extends Parent {
  ownedB: Arc<Bar>;
  constructor(a: Arc<Foo>, b: Arc<Bar>) { super(a); this.ownedB = b; }
}

const child = new Child(arcFoo, arcBar);
child[Symbol.dispose]();
```

`Object.getOwnPropertyNames(child)` returns `["ownedA", "ownedB"]` — both, because both were assigned in constructors via `this.x = ...`, making them own properties of the instance (not the prototype).

**PASS** — inherited fields that are assigned via `this.x = ...` in a constructor are own properties of the instance. They ARE enumerated.

BUT: If a parent class defines a property on the prototype (e.g., via a getter or a class field defined without assignment in the constructor), it would NOT be an own property of the instance. Example:

```typescript
class Parent extends Struct {
  get ownedA(): Arc<Foo> { return this._a; } // prototype property
}
```

`getOwnPropertyNames` would not find `ownedA` on the instance. However, this is an unusual pattern — class fields assigned in constructors are the norm and are safe.

### Verdict: 5a is a HIGH latent bug. 5b is PASS for normal usage.

---

## Scenario 6: Circular references. Struct A owns B, B owns Borrow<A>.

### Setup

```typescript
class B extends Struct {
  ref: Borrow<A>;
  constructor(a: A) { super(); this.ref = new Borrow(a); }
}

class A extends Struct {
  b!: B;
  init() { this.b = new B(this); }
}

const a = new A();
a.init();
a[Symbol.dispose]();
```

### Trace

1. `a[disposeSymbol]()`:
   - `#dropped = true`.
   - `a.drop()` → no-op.
   - Iterates own properties: `["b"]`.
   - `a.b` → B instance. Calls `b[disposeSymbol]()`.
2. Inside `b[disposeSymbol]()`:
   - `#dropped = true`.
   - `b.drop()` → no-op.
   - Iterates own properties: `["ref"]`.
   - `b.ref` → Borrow<A>. Calls `borrow[disposeSymbol]()` → **no-op**. Does NOT propagate back to A. **CORRECT.**
3. Back in A's cascade. No more properties. Done.

**No infinite loop. Borrow breaks the cycle correctly.** Even if Borrow DID propagate, the idempotency guard (`#dropped` check) on A would prevent re-entry.

### But what about a cycle WITHOUT Borrow?

```typescript
class X extends Struct {
  other!: Y;
}
class Y extends Struct {
  other!: X;
}
const x = new X();
const y = new Y();
x.other = y;
y.other = x;
x[Symbol.dispose]();
```

Trace:
1. `x[disposeSymbol]()`: `x.#dropped = true`. Iterates `["other"]`. Finds `y`. Calls `y[disposeSymbol]()`.
2. `y[disposeSymbol]()`: `y.#dropped = true`. Iterates `["other"]`. Finds `x`. Calls `x[disposeSymbol]()`.
3. `x[disposeSymbol]()`: `x.#dropped` is already true → **returns immediately**. Idempotency guard stops recursion.

**PASS** — the idempotency guard correctly prevents infinite recursion in ownership cycles. Though such cycles shouldn't exist in correct Rust-ported code, the JS implementation is defensively safe.

### Verdict: PASS

---

## Scenario 7: `const x = arc` vs `arc.clone()` — refcount bug?

### Setup

```typescript
class Payload extends Drop {
  drop() { console.log('Payload dropped'); }
}

const payload = new Payload();
const arc1 = Arc.new(payload);
const arc2 = arc1;           // BARE ASSIGNMENT — no clone!
// arc1 and arc2 are the same JS object. strongCount = 1.

arc1.drop(); // strongCount = 0 → payload.drop() fires
arc2.drop(); // strongCount is already 0 → early return (line 78: strongCount <= 0)
```

**Bug?**: `arc2.value` after `arc1.drop()` throws "inner value has been dropped" — but the developer might expect `arc2` to be a valid independent handle.

**This is a semantic trap, not a code bug.** The `Arc` class behaves correctly — it's the user's misunderstanding. But it IS dangerous because JS developers expect `const x = y` to work as a copy of a reference. The comment at arc.ts:9-11 documents this correctly.

**However**: consider this scenario:

```typescript
const arc1 = Arc.new(payload);
const arc2 = arc1; // bare assignment

// Pass arc2 to some function that drops it
someFunction(arc2); // internally calls arc2.drop()

// Now arc1 is also dead!
arc1.value; // throws!
```

This is the most likely real-world footgun. The Arc comment says "You MUST use arc.clone()" but there's no runtime enforcement. A lint rule is the only defense.

### Additional concern: Arc stored as struct field without clone

```typescript
class Holder extends Struct {
  data: Arc<Payload>;
  constructor(arc: Arc<Payload>) { super(); this.data = arc; }
}

const arc = Arc.new(payload);
const h = new Holder(arc); // h.data IS arc — same object, no clone

h[Symbol.dispose](); // cascades → arc.drop() → strongCount 0 → payload dropped
arc.value; // throws — but the caller still has a reference to `arc`!
```

**This is correct Rust semantics** — in Rust, passing `arc` to a struct constructor would be a move, consuming the original. The JS equivalent is that after constructing `Holder`, the caller should not use `arc` anymore. But JS doesn't enforce this.

### Verdict: KNOWN HAZARD — not a code bug, but a critical usage footgun

The code is correct. The hazard is in usage. A lint rule that forbids `= arcExpr` without `.clone()` is essential.

---

## Scenario 8: Enum with `{}` unit variant — does cascade work on empty objects?

### Setup

```typescript
type V = { Empty: {}; HasData: { x: Arc<Foo> } };
class MyEnum extends Enum<V> {
  static Empty() { return new MyEnum('Empty', {}); }
  static HasData(x: Arc<Foo>) { return new MyEnum('HasData', { x }); }
}

const e = MyEnum.Empty();
e[Symbol.dispose]();
```

### Trace

1. `Enum.[disposeSymbol]()` (enum.ts:40):
   - `this.isDropped` → false. Proceeds.
   - `Object.getOwnPropertyNames(this.value)` → `this.value` is `{}`. Returns `[]`. Empty array.
   - Loop body never executes. **OK.**
2. `super[disposeSymbol]()`:
   - `AkObject.[disposeSymbol]()`:
   - `this.drop()` → no-op.
   - `Object.getOwnPropertyNames(this)` → `["type", "value"]`.
   - `this.type` = `"Empty"` (string, no disposeSymbol). Skipped.
   - `this.value` = `{}` (plain object, no disposeSymbol). Skipped.
3. Done. No errors.

### Verdict: PASS

Unit variants with `{}` work correctly. The empty object has no properties to iterate.

---

## Scenario 9: Multiple [Symbol.dispose]() calls (idempotent?)

### 9a: AkObject

```typescript
const s = new Struct();
s[Symbol.dispose](); // first call
s[Symbol.dispose](); // second call
```

Trace:
1. First call: `#dropped` is false → sets true, runs drop(), cascades. **OK.**
2. Second call: `#dropped` is true → `return` immediately (object.ts:19). **IDEMPOTENT. PASS.**

### 9b: Arc

```typescript
const arc = Arc.new(payload);
arc[Symbol.dispose](); // first
arc[Symbol.dispose](); // second
```

Trace:
1. First: `arc.drop()` → strongCount 1 → 0. `dropped = true`. `payload.drop()` fires.
2. Second: `arc.drop()` → `strongCount <= 0` → `return` (arc.ts:78). **IDEMPOTENT. PASS.**

### 9c: Enum

```typescript
const e = MyEnum.HasData(Arc.new(foo));
e[Symbol.dispose]();
e[Symbol.dispose]();
```

Trace:
1. First: `this.isDropped` → false. Cascades value fields (disposes the Arc). Then `super[disposeSymbol]()` sets `#dropped = true`.
2. Second: `this.isDropped` → true → `return` (enum.ts:41). **IDEMPOTENT. PASS.**

### 9d: Borrow / BorrowMut

Always no-op. Multiple calls are trivially safe. **PASS.**

### 9e: Ref / RefMut (from RefCell)

```typescript
const cell = new RefCell(val);
const ref = cell.borrow();
ref[Symbol.dispose](); // first
ref[Symbol.dispose](); // second
```

`Ref` extends `Drop` which extends `AkObject`. First dispose sets `#dropped = true`, calls `ref.drop()` which calls `#release()` (decrements borrow count). Second dispose: `#dropped` is true → returns immediately. Does NOT double-decrement. **PASS.**

### Verdict: ALL IDEMPOTENT. PASS.

---

## Scenario 10: A Drop subclass forgets to call super.drop() — what breaks?

### Setup

```typescript
class Broken extends Drop {
  resource: SomeDisposable;

  drop(): void {
    this.resource.close();
    // FORGOT: super.drop()
  }
}
```

### Analysis

Let's look at what `super.drop()` would do. `Drop` extends `AkObject`. `AkObject.drop()` (object.ts:15) is a **no-op**: `drop(): void {}`.

So forgetting `super.drop()` in a Drop subclass loses... nothing. The base `drop()` does no work.

**But wait**: what about the cascade? The cascade is in `[disposeSymbol]()`, not in `drop()`. The call chain is:

1. `broken[disposeSymbol]()` → `AkObject.[disposeSymbol]()` (object.ts:18).
2. Sets `#dropped = true`.
3. Calls `this.drop()` → dispatches to `Broken.drop()` (the overridden one, with or without `super.drop()`).
4. **Then** cascades via `getOwnPropertyNames` loop.

The cascade happens in `[disposeSymbol]()`, which is **not overridden** by `Broken`. It always runs regardless of what `drop()` does. The `drop()` method is purely for custom cleanup — it doesn't need to call `super.drop()` because the base is a no-op and the cascade runs independently.

**What if someone overrides `[disposeSymbol]()` instead of `drop()`?**

```typescript
class ReallyBroken extends Drop {
  [disposeSymbol](): void {
    // custom cleanup
    // FORGOT: super[disposeSymbol]()
  }
}
```

THIS would be catastrophic:
- `#dropped` never set → idempotency broken
- `leakRegistry.unregister` never called → false leak warning on GC
- Auto-cascade never runs → owned fields not disposed
- `drop()` never called → custom cleanup in `drop()` skipped

But overriding `[disposeSymbol]()` directly should be extremely rare. The API design encourages overriding `drop()` instead. The `Drop` abstract class enforces `abstract override drop(): void` — pushing users toward the correct pattern.

**The Enum class overrides `[disposeSymbol]()`** (enum.ts:40) and DOES call `super[disposeSymbol]()` (enum.ts:48). This is correct. But it demonstrates that framework code does override `[disposeSymbol]()`, so the risk exists.

### Verdict: PASS for `drop()` (forgetting `super.drop()` is harmless). HIGH RISK if someone overrides `[disposeSymbol]()` without calling super.

---

## Additional Adversarial Findings

### Finding 11: Arc.drop() calls value.drop() but NOT value[disposeSymbol]()

In `arc.ts:82-86`:
```typescript
if (val && typeof (val as any).drop === 'function') {
  (val as any).drop();
}
```

This calls `.drop()` directly, **not** `[disposeSymbol]()`. This means:
- The inner value's custom `drop()` fires. **OK.**
- The inner value's **auto-cascade does NOT fire**. The inner value's owned fields are NOT recursively disposed.
- The inner value's `#dropped` flag is NOT set (that's done in `[disposeSymbol]()`).
- The inner value's `leakRegistry.unregister()` is NOT called → false leak warning.

**BUG: CRITICAL** — If the inner value of an Arc is an AkObject (Struct, Enum, etc.), dropping the last Arc calls `.drop()` but skips the auto-cascade and the idempotency/leak-registry bookkeeping.

**Example**:
```typescript
class Inner extends Struct {
  child: Arc<Grandchild>;
  constructor(gc: Arc<Grandchild>) { super(); this.child = gc; }
  // no drop() override — relies on auto-cascade
}

const gc = Arc.new(new Grandchild());
const inner = new Inner(gc);
const arc = Arc.new(inner);
arc.drop(); // calls inner.drop() which is no-op!
// inner.child (the Arc<Grandchild>) is NEVER disposed!
// inner is never marked as dropped!
// leakRegistry still thinks inner is alive!
```

**Fix**: Arc.drop() should call `value[disposeSymbol]()` instead of `value.drop()`:

```typescript
if (val && typeof (val as any)[disposeSymbol] === 'function') {
  (val as any)[disposeSymbol]();
}
```

### Finding 12: Arc does not extend AkObject — no leak detection for the Arc handle itself

`Arc<T>` is a plain class, not an AkObject subclass. It's not registered with the `leakRegistry`. If an Arc handle is created but never dropped (and never GC'd while registered), there's no leak warning for the Arc handle itself.

The inner value might be registered (if it extends AkObject), but the Arc wrapper isn't. This means:

```typescript
const arc = Arc.new(someValue);
// forget to drop arc, let it go out of scope
// No leak warning for the arc handle
```

In Rust, dropping an Arc that goes out of scope is automatic (RAII). In JS, without leak detection on the Arc handle itself, leaked Arcs are silent.

**Severity**: MEDIUM — the leak detection system has a blind spot for Arc handles.

**Recommendation**: Either register Arc handles with the leak registry, or document that Arc handles are not leak-detected and must be manually tracked.

### Finding 13: Enum disposes value fields THEN calls super (potential use-after-dispose in drop())

In `enum.ts:40-48`:
```typescript
override [disposeSymbol](): void {
  if (this.isDropped) return;
  // First: dispose value fields
  for (const key of Object.getOwnPropertyNames(this.value)) { ... }
  // Then: call super which calls this.drop()
  super[disposeSymbol]();
}
```

The value fields are disposed **before** `this.drop()` is called (via super). If a subclass's `drop()` method accesses `this.value.someField`, that field may already be disposed.

Compare with `AkObject[disposeSymbol]()` (object.ts:18-29) which calls `this.drop()` **before** cascading to fields. The Enum override reverses this order.

**This violates Rust's drop order**: In Rust, `drop()` runs first, then fields are dropped in declaration order. The Enum's order is inverted.

**Severity**: MEDIUM — only matters if an Enum subclass's `drop()` accesses value fields.

### Finding 14: Weak.upgrade() uses `new (Arc as any)(this.#inner)` — bypasses type safety

In `arc.ts:139`:
```typescript
return new (Arc as any)(this.#inner);
```

This works because it's casting to bypass the `private constructor`. But it also bypasses any future constructor validation that might be added to Arc. The `as any` cast is a maintenance hazard.

**Severity**: LOW — works correctly today, but fragile.

### Finding 15: DropGuard is dead code / has no integration path

`DropGuard` (drop.ts:11-31) is defined but never used by any of the existing types. `AkObject` has its own `#dropped` flag and leak registration. `Drop` extends `AkObject` and inherits that machinery. `DropGuard` duplicates this functionality for... what? Types that don't extend AkObject?

If `DropGuard` is intended for Arc or other non-AkObject types, it's not being used there either. Arc has its own dropped flag on the inner.

**Severity**: LOW — dead code, not a bug. May cause confusion.

---

## Summary

| # | Scenario | Verdict | Severity |
|---|----------|---------|----------|
| 1 | Struct with Arc + Borrow, drop struct | PASS | — |
| 2 | Enum Arc extract/clone/drop | PASS | — |
| 3 | Arc → Weak → drop all strong → upgrade | PASS | — |
| 4 | Nested Struct → Struct → Arc<Drop> | PASS | — |
| 5a | Private `#` fields invisible to cascade | **LATENT BUG** | HIGH |
| 5b | Inherited fields | PASS | — |
| 6 | Circular refs with Borrow | PASS | — |
| 7 | Bare assignment vs .clone() | KNOWN HAZARD | HIGH (usage) |
| 8 | Unit variant `{}` | PASS | — |
| 9 | Multiple dispose calls | PASS (all idempotent) | — |
| 10 | Forgetting super.drop() | PASS (base is no-op) | — |
| 11 | **Arc.drop() calls .drop() not [disposeSymbol]()** | **BUG** | **CRITICAL** |
| 12 | Arc not leak-detected | DESIGN GAP | MEDIUM |
| 13 | Enum dispose order inverted vs Rust | **BUG** | MEDIUM |
| 14 | Weak.upgrade() uses `as any` cast | FRAGILE | LOW |
| 15 | DropGuard is dead code | UNUSED | LOW |

### Critical Action Items

1. **Finding 11 (CRITICAL)**: `Arc.drop()` must call `[disposeSymbol]()` on the inner value, not `.drop()`. Currently, dropping the last Arc on an AkObject-derived inner skips auto-cascade, leak-registry cleanup, and idempotency bookkeeping. This is the most serious bug found.

2. **Finding 5a (HIGH)**: Document and/or lint-enforce that AkObject subclasses must not store owned disposables in ECMAScript `#` private fields. Use TypeScript `private` keyword instead (which produces enumerable properties in some configurations) or manually dispose in `drop()`.

3. **Finding 13 (MEDIUM)**: Enum's `[disposeSymbol]()` should call `super[disposeSymbol]()` first (which runs `drop()` and the AkObject cascade on the enum's own properties), then dispose value fields — or restructure to match Rust's drop order (custom drop first, then fields).
