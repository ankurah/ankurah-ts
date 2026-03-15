# Ownership: Rust to TypeScript

**Goal**: Translated TS code should read as close to the Rust source as possible. All types live in `@ankurah/base`.

---

## Type Hierarchy

Every ported Rust type extends `AkObject`, which provides automatic drop glue — when disposed, it cascades to all owned fields.

```
AkObject          — base for all ported types, auto-cascade [Symbol.dispose]()
  ├── Struct      — ported Rust structs
  ├── Enum<V>     — ported Rust enums (match(), is(), typed variants)
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
| `&T` | `Borrow<T>` | Non-owning. `[Symbol.dispose]()` is a no-op — does NOT cascade. |
| `&mut T` | `BorrowMut<T>` | Non-owning mutable. Same no-op dispose. |
| `Box<T>` | `T` (plain) | Unique ownership. Cascade handles it. |
| `Mutex<T>` | `Mutex<T>` | `using guard = mutex.lock()`. |
| `RwLock<T>` | `Mutex<T>` | No reader/writer distinction in JS. |
| `RefCell<T>` | `RefCell<T>` | `borrow()` / `borrow_mut()` returning guards. |
| `tokio::sync::Mutex` | `AsyncMutex` | Async serialization across `await` points. |
| `AtomicBool` / `AtomicU32` | `boolean` / `number` | Single-threaded JS. |
| Lifetimes (`'a`, `'rec`) | Runtime `alive` flag | Check at mutation points. |
| `fn method(self)` (move) | Runtime `alive` flag | JS has no move semantics. |

## Ownership Semantics

**Default = owned.** A plain field on a Struct/Enum is owned. The cascade drops it automatically.

**`Borrow<T>` / `BorrowMut<T>`** — marks a field as NOT owned. The cascade calls their `[Symbol.dispose]()` which is a no-op. The lint rule uses this to verify cascade correctness.

**`Arc<T>`** — shared ownership. Multiple owners hold clones. Inner value drops when the last Arc drops (refcount = 0). `arc.clone()` increments. `arc.drop()` decrements. Bare `const x = arc` does NOT increment — always use `.clone()`.

## Drop Glue

`[Symbol.dispose]()` on AkObject is the drop glue. It:
1. Sets `#dropped` flag (idempotent)
2. Unregisters from FinalizationRegistry
3. Calls `this.drop()` (custom cleanup — no-op unless type extends `Drop`)
4. Cascades: walks all own properties, calls `[Symbol.dispose]()` on each

`Enum` overrides cascade to also walk `this.value`'s properties (variant data fields).

`using` calls `[Symbol.dispose]()` at block exit. That's the normal path.

## FinalizationRegistry

Leak detection only. If an AkObject (or Arc) is GC'd without being disposed, FR fires a warning with the class name and creation stack trace.

## Async Serialization

`Mutex<T>` absorbs `std::sync::Mutex` (trivial in single-threaded JS). `AsyncMutex` replaces `tokio::sync::Mutex` (async serialization across `await` points).

## Inherent Limitations

- **No compile-time lifetimes** — `alive` flags + `assertNotDropped()` + lint are runtime-only.
- **FR non-determinism** — may fire late or never. Lint enforcing `using` closes gap at dev time.
- **No move semantics** — `alive` flag is the only protection against use-after-consume.
- **`const x = arc` footgun** — bare assignment doesn't increment refcount. Lint should flag this.
