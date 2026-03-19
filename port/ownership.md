# Ownership: Rust to TypeScript

**Goal**: Translated TS code should read as close to the Rust source as possible. Ownership types live in `@ankurah/base`.

---

## Core Principle

Every ported Rust type extends `AkObject`, which provides automatic drop cascade and leak detection. This mirrors Rust's automatic drop semantics — when a value goes out of scope, its fields are dropped recursively.

**The key distinction in TS is `using` vs `const`:**
- `using x = new Foo()` — block-scoped ownership. Disposed at block exit. Mirrors Rust's implicit drop at scope end.
- `const x = new Foo()` — the value will be stored or returned. Someone else owns it. Mirrors Rust's move semantics.

If a `const` AkObject goes out of scope without being stored as a field on another AkObject (where cascade would handle it), the leak detector fires a **fatal error** with the creation stack trace.

---

## Type Hierarchy

```
AkObject          — base for all ported types, auto-cascade [Symbol.dispose]()
  ├── Struct      — ported Rust structs (struct Foo → class Foo extends Struct)
  ├── Enum<V>     — ported Rust enums (match(), is(), typed variants, cascade into variant fields)
  └── Drop        — types with `impl Drop` (override drop() for custom cleanup)
```

## Mapping Rules

| Rust | TS | Notes |
|------|-----|-------|
| `struct Foo` | `class Foo extends Struct` | Auto-cascade drops owned fields. |
| `enum Foo` | `class Foo extends Enum<V>` | Typed variants, `match()`, cascade into variant fields. |
| `impl Drop for T` | `class T extends Drop` | Override `drop()` for custom cleanup. |
| `Arc<T>` | `Arc<T>` | Refcounted shared ownership. Inner drops when last Arc drops. |
| `Rc<T>` | `Arc<T>` | Same (no threading distinction in JS). |
| `Weak<T>` | `Weak<T>` | `upgrade()` returns `Arc<T> \| null`. |
| `&T` (in fields) | `Borrow<T>` | Non-owning. `[Symbol.dispose]()` is a no-op — does NOT cascade. |
| `&mut T` (in fields) | `BorrowMut<T>` | Non-owning mutable. Same no-op dispose. |
| `Box<T>` | `T` (plain) | Unique ownership. Cascade handles it. |
| `Mutex<T>` | `Mutex<T>` | `using guard = mutex.lock()`. |
| `RwLock<T>` | `Mutex<T>` | No reader/writer distinction in JS. |
| `RefCell<T>` | `RefCell<T>` | `borrow()` / `borrow_mut()` returning guards. |
| `tokio::sync::Mutex` | `AsyncMutex` | Async serialization across `await` points. |
| `AtomicBool` / `AtomicU32` | `boolean` / `number` | Single-threaded JS. |
| Lifetimes (`'a`, `'rec`) | Runtime `alive` flag | Check at mutation points. |
| `fn method(self)` (move) | Runtime `alive` flag | JS has no move semantics. |

## Ownership Semantics

### `using` vs `const` — The Core Decision

In Rust, every local variable is automatically dropped at scope exit unless moved. TS has no automatic drop, so you make the decision explicitly:

```typescript
// Block-scoped: disposed at block exit (like Rust's implicit drop)
{
  using entity = await node.get(id);
  // ... use entity ...
} // entity[Symbol.dispose]() called here — cascades to all owned fields

// Stored/returned: someone else owns this (like Rust's move)
const entity = await node.get(id);
this.cachedEntity = entity; // parent's cascade will dispose it later
```

**Rule of thumb**: if the value doesn't leave the block, use `using`. If it's stored as a field or returned, use `const`.

### Cascade Disposal

`[Symbol.dispose]()` on AkObject is the drop glue. It:
1. Sets `#dropped` flag (idempotent)
2. Unregisters from FinalizationRegistry
3. Calls `this.drop()` (custom cleanup — no-op unless type extends `Drop`)
4. Cascades: walks all own properties, calls `[Symbol.dispose]()` on each
5. Recurses into arrays: if a field is an `Array`, disposes each disposable element

`Enum` overrides cascade to also walk `this.value`'s properties (variant data fields), including arrays.

This means stored fields are automatically disposed when their parent is disposed. You don't need to manually track them.

### Non-Owning References

**`Borrow<T>` / `BorrowMut<T>`** — marks a field as NOT owned. The cascade calls their `[Symbol.dispose]()` which is a no-op, preventing accidental destruction of borrowed values.

### Shared Ownership

**`Arc<T>`** — shared ownership with refcounting. Has its own leak detection (independent of AkObject).

- `Arc.new(value)` — creates with refcount 1
- `arc.clone()` — increments refcount, returns new Arc
- `arc.drop()` / `using` — decrements refcount; drops inner when refcount hits 0
- `const x = arc` does **NOT** increment refcount — always use `.clone()`

## FinalizationRegistry

Leak detection. If an AkObject (or Arc) is GC'd without being disposed, FinalizationRegistry throws a **fatal error** with the class name and creation stack trace. This catches real ownership bugs — values that went out of scope without `using` and weren't stored on a parent.

## Async Serialization

`Mutex<T>` absorbs `std::sync::Mutex` (trivial in single-threaded JS). `AsyncMutex` replaces `tokio::sync::Mutex` (async serialization across `await` points).

## Inherent Limitations

- **No compile-time lifetimes** — `alive` flags + `assertNotDropped()` + lint are runtime-only.
- **FR non-determinism** — may fire late or never. Lint enforcing `using` closes gap at dev time.
- **No move semantics** — `alive` flag is the only protection against use-after-consume.
- **`const x = arc` footgun** — bare assignment doesn't increment refcount. Lint should flag this.
