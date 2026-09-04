# Ownership: Rust to TypeScript

**Goal**: translated TS reads as close to the Rust source as possible, and behaves
the same. The ownership types live in `@ankurah/base` (`packages/base/src`).

Rust's compiler enforces ownership before the program runs. TypeScript cannot, so
the polyfills enforce the same rules while it runs: they drop what Rust drops, in
the order Rust drops it, and they stop the program when they see a state the Rust
compiler would have refused to build.

---

## The model

Every ported Rust type extends `AkObject`. An `AkObject` is alive, dropped, or
moved, and it is registered with the leak detector from construction until one of
the latter two happens.

**Dropping** releases a value and everything it owns. `AkObject.drop()` is the
whole template, and nothing overrides it:

1. refuse if this value was already dropped or moved;
2. mark dropped and leave the leak registry;
3. call `onDrop()` — the type's own cleanup, with every field still alive;
4. in a `finally`, drop each of `ownedFields()`.

Step 3 before step 4 is not an implementation detail: Rust runs `Drop::drop`
before dropping fields, so a cleanup body that reads a field must still find it
usable.

**Moving** hands a value's contents to somebody else. A moved value is not
dropped and is not a leak — there is nothing left in it to release — and every
later use of it is fatal. `Result`'s `self`-taking methods move, and a released
`Arc` handle reports use-after-move for the same reason: the handle was this
scope's owner, and in Rust the moved-out binding is no longer nameable. A
consuming `match` for enums comes with the emitter.

**The cascade** walks what a value owns. `dropOwned(v)` drops anything with a
`drop()` method, walks arrays, Maps (keys *and* values, because `HashMap<K, V>`
owns both), Sets and plain objects to any depth, and lets primitives go. An `Arc`
ends the walk: it decrements, and only the last strong drop cascades into the
contents. Reaching the same object twice in one cascade is aliased ownership and
the second drop reports it.

---

## What the transpiler emits against

**`impl Drop for T` becomes `protected override onDrop()`.** Never an override of
`drop()`. Emitting `drop()` puts the cleanup after the cascade, which is the
wrong order and hands the body dead fields.

**Fields are dropped in the order the constructor assigned them.** The cascade
walks own properties, and `Object.getOwnPropertyNames` returns identifier keys in
insertion order, so the emitter must assign fields in declaration order to match
Rust's field drop order. (Integer-like keys — `"0"`, `"1"` — sort ahead of all
identifiers, so the guarantee holds for the field names the emitter produces, not
for an object keyed by numbers.)

**A type that keeps state in `#private` fields overrides `ownedFields()`.** The
cascade cannot see private state by walking properties. `Enum` does this for its
variant payload; the containers do it for their contents.

**A value owned by a block is dropped in a `finally`.**

```typescript
const entity = await node.get(id);
try {
  // ... use entity ...
} finally {
  entity.drop();
}
```

**A guard temporary is released at the end of its statement and again in the
enclosing `finally`.** That is why a guard's second drop is deliberately a no-op,
and it is the only place in the runtime where a second drop is not fatal. Nothing
but `Guard` in `std/guard.ts` overrides `drop()`, and the reason is written next
to it.

**`Copy` types have no drop glue.** A `Copy` type cannot implement `Drop` in
Rust, so the emitter gives it no `drop()` method and no registry entry — whatever
class shape it picks for it is the emitter's decision. The runtime tests for the
absence of drop glue, not for "is it a primitive". That is what makes
`guard.value = 5`, or re-storing a `Copy` struct, legal, while re-storing an
owned object the container already holds is fatal.

**`thread_local!` becomes `ThreadLocal`.** A static that lives for the whole
program: never a struct field, never dropped, not leak-tracked.

**A Rust function returning `Result` returns a `Result` value.** It does not
throw. `Result`'s `self`-taking methods (`unwrap`, `expect`, `map`, `andThen`,
`ok`, …) consume the receiver, exactly as Rust does; `isOk`/`isErr` borrow.
`Option<T>` is `T | null`.

---

## Fatal, panic, and the latch

**Fatal** is for anything Rust rejects at compile time. It means the emitter is
wrong, and every one goes through `fatal()` in `drop_registry.ts`:

| Condition | Reported by |
|---|---|
| Dropping a value twice | `fatalDoubleDrop` |
| Using a dropped value | `fatalUseAfterDrop` |
| Using a moved value | `fatalUseAfterMove` |
| Dropping a container while a guard on it is outstanding | `fatalOutstandingGuard` |
| Assigning a container the object it already holds | `fatalSelfAssignment` |
| A match with a missing arm | `fatalNonExhaustiveMatch` |
| Collecting a value that was never dropped | the leak registry |

**A plain throw** is for anything the emitted code got right but the program got
wrong: `RefCell` borrow conflicts and `unwrap()` on an `Err`, both of which panic
in Rust too. Re-locking a `std::sync::Mutex` on one thread is the exception that
proves the rule — Rust deadlocks rather than panicking, so this throws instead,
reporting the same bug where a hang would be undiagnosable.

**The poison latch.** `fatal()` sets a latch before it throws, and every liveness
check reads the latch first. A host can swallow a throw — an `uncaughtException`
handler, a browser page that keeps painting — and the program would then run on
over corrupted ownership. After the first fatal, the next check refuses instead.

**Every fatal is thrown as an `OwnershipFatal`** (exported from the package), so
emitted code can tell an ownership bug from a Rust error value. A `catch` block
that handles a Rust error type must test for `OwnershipFatal` and rethrow it
unconditionally — the runtime has already found something Rust would not have
compiled, and nothing after it can be trusted.

**`setOnFatal(handler)`** replaces what a fatal does, for a host that has to stop
differently — killing a worker, failing one request. The default throws an
`OwnershipFatal`.

**Tests**: the root `bunfig.toml` preloads `packages/base/src/testing.ts`, and
loading it installs the hooks — a suite needs no setup of its own. They reset the
latch before each test and fail any test that raised a fatal and swallowed it.
Without the reset, one test's fatal fails every test after it and the failure
lands nowhere near the bug. A test that provokes a fatal on purpose asserts it
and then calls `clearFatalLatch()` to acknowledge it.
`installOwnershipTestHooks()` is idempotent, so a file may call it explicitly to
make the dependency visible.

---

## Leak detection

Every `AkObject`, every `Arc` and `Weak` handle, and every container (`Mutex`,
`RwLock`, `RefCell`, `AsyncMutex`) is registered at construction and unregistered
when dropped or moved. A registered value that is garbage collected was never
dropped, and that is fatal, reported from a microtask because a
FinalizationRegistry callback has no caller to throw to.

`FinalizationRegistry` is feature-detected at module load. Where it is missing the
runtime installs a no-op registry and warns once, loudly: every other ownership
check still works, but a value that is simply forgotten goes unreported.

**Hermes**: `FinalizationRegistry` support landed on the `static_h` branch and
shipped in `260318099.0.0` — see facebook/hermes issue 1440, comment of
2026-04-30 by lavenzg ("FinalizationRegistry support has been landed in the
static_h branch ... included in the staging branch (260318099.0.0-staging)"), and
the Hermes release note of 2026-06-05 for `260318099.0.0`. Expo Go builds older
than that run the port with leak detection off, which is why a missing registry
has to be a warning the port survives rather than a crash.

Allocation stacks cost about a microsecond per construction, so they are behind
`setCaptureStacks(enabled)`, on by default except when `NODE_ENV=production`.
Labels are always recorded, so a report always names the type even without a
stack.

---

## Type mapping

| Rust | TS | Notes |
|------|-----|-------|
| `struct Foo` | `class Foo extends Struct` | Cascade drops owned fields. |
| `enum Foo` | `class Foo extends Enum<V>` | `match()`, `is()`, typed variants; cascade drops the payload. |
| `impl Drop for T` | `protected override onDrop()` | Runs before the fields are dropped. |
| `Arc<T>` / `Rc<T>` | `Arc<T>` | Refcounted. `arc.clone()` — a bare assignment does **not** increment. Inner drops when the last handle drops. |
| `Weak<T>` | `Weak<T>` | `upgrade()` returns `Arc<T> \| null`. Tracked and dropped like anything else. |
| `&T` / `&mut T` in fields | `Borrow<T>` / `BorrowMut<T>` | Non-owning; the cascade steps over them. |
| `Box<T>` | `T` | Unique ownership; the cascade handles it. |
| `Mutex<T>` | `Mutex<T>` | `lock()` returns a `MutexGuard<T>`. |
| `RwLock<T>` | `RwLock<T>` | Its own type: `read()` and `write()` return distinct guards. |
| `RefCell<T>` | `RefCell<T>` | `borrow()` / `borrowMut()`, with Rust's runtime borrow rules. |
| `tokio::sync::Mutex<T>` | `AsyncMutex<T>` | Serializes across `await`; `acquire()` returns a guard. |
| `Option<T>` | `T \| null` | |
| `Result<T, E>` | `Result<T, E>` | A returned value, not a throw. |
| `thread_local!` | `ThreadLocal<T>` | Static; not tracked. |
| `AtomicBool` / `AtomicU32` | `boolean` / `number` | Single-threaded JS. |
| Lifetimes (`'a`) | runtime liveness checks | No compile-time lifetimes. |
| `fn method(self)` (move) | consuming method + moved state | No move semantics in JS. |

A container owns its contents: dropping a `Mutex<T>` drops the `T`, as in Rust.
A guard does not own what it points at — it reads and writes through the
container's own storage, so `*guard = v` replaces what the container holds and
drops what was there.

---

## Inherent limitations

- **No compile-time lifetimes.** Every check here is a runtime check, so a path
  that never runs is never checked.
- **FinalizationRegistry is non-deterministic.** A leak may be reported late, or
  not at all, and never on a host without the registry.
- **No move semantics.** The moved flag is the only thing standing between a
  consumed value and its next use.
- **`const x = arc` does not increment the refcount.** Only `arc.clone()` does.
- **The cascade cannot see `#private` fields**, which is why `ownedFields()`
  exists.
- **Foreign objects.** A class instance with no drop glue that the cascade
  reaches gets one `console.warn` per constructor name: whatever it owns will not
  be released, and it needs a provided type to wrap it.

## Retired

`using` declarations and `[Symbol.dispose]` were the original model. They are
retired: Hermes refuses `using`, so the transpiler emits explicit `.drop()` calls
and `try/finally` blocks instead. `[Symbol.dispose]` still exists on `AkObject`
and `Arc` and still delegates to `drop()`, but it is not the model and no new
code should reach for it; the cascade dispatches on `drop()`.
