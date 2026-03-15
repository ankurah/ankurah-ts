# Memory Model: Rust Ownership to TypeScript GC

**Goal**: Translated TS code should read as close to the Rust source as possible while preserving equivalent semantics. Provided types absorb JS-specific complexity so translation stays 1:1.

See also: [decisions.md](decisions.md), [provided-types.md](provided-types.md), [lint-rules.md](lint-rules.md).

---

## Mapping Rules

| Rust | TS | Notes |
|------|-----|-------|
| `impl Drop for T` | `extends Disposable` | `onDispose()` = Drop body. Used with `using`. |
| Owns Drop-implementing fields | Same — `onDispose()` disposes owned fields | JS has no auto-cascade. |
| `Arc<T>` / `Rc<T>` | `T` (delete wrapper) | GC provides shared ownership. |
| `Weak<T>` | `WeakRef<T>` | `deref()` must handle `undefined`. |
| `Mutex<T>` | `Mutex<T>` | 1:1 provided type. `using guard = mutex.lock()`. |
| `RwLock<T>` | `Mutex<T>` | No reader/writer distinction needed in JS. |
| `MutexGuard<T>` | `MutexGuard<T>` (Disposable) | Drop side-effects fire in `onDispose()`. |
| `RefCell<T>` | `RefCell<T>` | 1:1 provided type. `borrow()` / `borrow_mut()`. |
| `Ref<T>` / `RefMut<T>` | `Ref<T>` / `RefMut<T>` (Disposable) | `using guard = cell.borrow_mut()`. |
| `tokio::sync::Mutex` | `PromiseMutex` | Async serialization across `await` points. |
| `AtomicBool` / `AtomicU32` | `boolean` / `number` | Single-threaded JS. |
| Lifetimes (`'a`, `'rec`) | Runtime `alive` flag | Check at mutation points; set `false` on consume. |
| `fn method(self)` (move) | Runtime `alive` flag | JS has no move semantics. |

---

## Disposal

`impl Drop` → `Disposable` + `using`. Classify each type by severity:

- **Correctness-critical** (missed cleanup = silent wrong behavior): FR **crashes hard** with file+line.
- **Resource hygiene** (missed cleanup = waste, nobody sees wrong data): FR **warns**.

Test: "if cleanup never runs, does anyone see wrong data?" Yes → crash. No → warn.

### Enforcement

1. **Lint** — catches missing `using` at dev time
2. **`assertNotDisposed()`** — catches use-after-dispose at call time
3. **`FinalizationRegistry`** — catches forgot-to-dispose at GC time

FR is a diagnostic backstop, not a cleanup mechanism. If `onDispose()` throws, the object is still considered disposed and FR is unregistered — the throw propagates to the caller.

---

## Async Serialization

`std::sync::Mutex` semantics are absorbed by the provided `Mutex<T>` (trivial in single-threaded JS). `tokio::sync::Mutex` → `PromiseMutex` (async serialization still matters).

**Rule**: if no `await` between read and write, no protection needed. If fire-and-forget async tasks mutate shared state, `PromiseMutex` is required.

---

## WeakRef

`Weak<T>` → `WeakRef<T>`, like-for-like. `deref()` must handle `undefined`. Strong holder must exist. FR cleans stale map entries.

---

## Inherent Limitations

- **No compile-time lifetimes** — `alive` flags + `assertNotDisposed()` + lint are runtime-only.
- **FR non-determinism** — may fire late or never. Lint enforcing `using` closes gap at dev time.
- **No move semantics** — `alive` flag is the only protection against use-after-consume.
