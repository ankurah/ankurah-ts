# Provided Types — API Reference

**Source location**: `packages/base/src/`

These types implement the ownership model described in [ownership.md](../ownership.md),
which is the contract; this file is the API surface. Every ported Rust type
extends one of these base classes.

Rewritten 2026-09-02 against the runtime as it stands. The version before it
described `using` declarations, `[Symbol.dispose]` as the drop mechanism, a
`RefCell.borrow_mut()` that is now `borrowMut()`, and an `AsyncMutex.run(fn)`
that never existed. See [../retractions-2026-09-02.md](../retractions-2026-09-02.md).

---

## AkObject

Base class for all ported Rust types. Registers with the leak detector at
construction, and drops what it owns.

### API

```typescript
class AkObject {
    constructor(label?: string);      // registers with the leak registry
    drop(): void;                     // the whole drop template — never overridden
    get isDropped(): boolean;
    get isMoved(): boolean;
    protected get label(): string;
    protected onDrop(): void;         // the type's own cleanup — override this
    protected ownedFields(): unknown[];  // what the cascade drops; default is every own property
    protected assertNotDropped(): void;
    protected markMoved(): void;
    [disposeSymbol](): void;          // delegates to drop()
}
```

### What `drop()` does, in order

1. refuse if this value was already dropped or moved — both are fatal;
2. mark dropped and leave the leak registry;
3. call `onDrop()`, with every field still alive;
4. in a `finally`, drop each of `ownedFields()`.

Step 3 before step 4 mirrors Rust, which runs `Drop::drop` before dropping
fields. **Override `onDrop()`, never `drop()`.** An override of `drop()` puts the
cleanup after the cascade and hands the body dead fields.

`ownedFields()` returns every own property by default. A type that keeps owned
state in `#private` fields must override it, because the cascade cannot see
private state.

### Cascade behavior

`dropOwned(v)` — exported — drops anything with a `drop()` method, walks arrays,
Maps (keys *and* values, because `HashMap<K, V>` owns both), Sets and plain
objects to any depth, and lets primitives go. Reaching the same object twice in
one cascade is aliased ownership, and the second drop reports it.

Fields marked with the module-private `nonOwning` symbol are stepped over:
`Borrow`, `BorrowMut` and `ThreadLocal` carry it. A class instance the cascade
reaches that has no drop glue and no marker gets one `console.warn` per
constructor name — whatever it owns will not be released, and it needs a
provided type to wrap it.

---

## Struct

Base class for ported Rust structs. Empty subclass of AkObject.

```typescript
class Struct extends AkObject {}
```

Usage: `struct Foo` → `class Foo extends Struct`.

---

## Enum\<V\>

Base class for ported Rust enums. Keeps its variant and payload in `#private`
state and hands the payload to the cascade through `ownedFields()`.

### API

```typescript
class Enum<V extends Record<string, object>> extends AkObject {
    constructor(type: string & keyof V, value: V[keyof V]);
    get type(): string & keyof V;      // asserts the value is alive
    get value(): V[keyof V];           // asserts the value is alive
    match<R>(arms: { [K in keyof V]: (value: V[K]) => R }): R;
    is<K extends keyof V>(variant: K): boolean;
    toString(): string;                // safe on a dropped or moved value
}
```

`match()` borrows: it does not consume the enum. A missing arm is fatal — Rust
would not have compiled a non-exhaustive match. A consuming `match` comes with
the emitter.

### Usage

```typescript
type DeltaContentV = {
  StateSnapshot: { state: StateFragment };
  EventBridge: { events: EventFragment[] };
};

class DeltaContent extends Enum<DeltaContentV> {
  // impl methods
}

// Construction
new DeltaContent('StateSnapshot', { state })

// Matching
content.match({
  StateSnapshot: v => handleState(v.state),
  EventBridge: v => handleEvents(v.events),
})

// Type guard
if (content.is('StateSnapshot')) { content.value.state ... }
```

---

## Result\<T, E\>

Maps `Result<T, E>`. **A returned value, not a throw.** `Option<T>` is `T | null`
and needs no type.

### API

```typescript
class Result<T, E> extends Enum<ResultV<T, E>> {
    static Ok<T, E = never>(value: T): Result<T, E>;
    static Err<T = never, E = unknown>(error: E): Result<T, E>;

    isOk(): boolean;                   // borrows
    isErr(): boolean;                  // borrows

    unwrap(): T;                       // the rest consume the receiver
    unwrapErr(): E;
    expect(message: string): T;
    expectErr(message: string): E;
    unwrapOr(defaultValue: T): T;
    unwrapOrElse(f: (err: E) => T): T;
    map<U>(f: (value: T) => U): Result<U, E>;
    mapErr<F>(f: (err: E) => F): Result<T, F>;
    andThen<U>(f: (value: T) => Result<U, E>): Result<U, E>;
    orElse<F>(f: (err: E) => Result<T, F>): Result<T, F>;
    ok(): T | null;
    err(): E | null;
}
```

The `self`-taking methods consume the receiver, exactly as Rust does: the
`Result` is left moved and every later use of it is fatal. They also drop what
they do not hand back — `unwrapOr` drops the default on an `Ok`, and a callback
that throws drops the payload rather than stranding it.

`unwrap()` on an `Err` throws, because it panics in Rust.

---

## Drop

Abstract subclass of AkObject for types with `impl Drop` in Rust. Forces you to
implement the cleanup hook.

### API

```typescript
abstract class Drop extends AkObject {
    protected abstract onDrop(): void;   // implement cleanup here
}
```

### Usage

```typescript
class Transaction extends Drop {
    #committed = false;

    protected onDrop(): void {
        if (!this.#committed) {
            this.rollback();
        }
    }
}

// A block-owned value is dropped in a finally:
const trx = node.beginTransaction();
try {
  // ...
} finally {
  trx.drop();
}
```

---

## DropGuard

Composition helper for types that need leak detection but do not extend
`AkObject` — the containers use it, since a container is not itself a ported
Rust value.

### API

```typescript
class DropGuard {
    constructor(host: object, label?: string);  // registers host with the leak registry
    markDropped(host: object): void;            // call from the host's drop()
    assertNotDropped(): void;
    get isDropped(): boolean;
}
```

---

## Guard / ReadGuard\<T\> / WriteGuard\<T\>

The bases every lock and borrow guard extends. A guard does not own what it
points at: it reads and writes through the container's own storage, so
assigning through a write guard replaces what the container holds and drops what
was there.

### API

```typescript
abstract class Guard extends Drop {
    drop(): void;                      // idempotent: a second drop is a no-op
}

abstract class ReadGuard<T> extends Guard {
    get value(): T;
}

abstract class WriteGuard<T> extends ReadGuard<T> {
    set value(v: T);                   // drops what the container held
}
```

`Guard` is the one place in the runtime where a second drop is not fatal, and it
is deliberate: the emitter releases a guard temporary at the end of its
statement and again in the enclosing `finally`. Nothing but `Guard` overrides
`drop()`.

Assigning a container the object it already holds is fatal — that is aliased
ownership. Re-storing a `Copy`-like value is not, because a value with no drop
glue cannot be aliased in a way that matters.

---

## Arc\<T\> / Weak\<T\>

Refcounted shared ownership. Both handles are leak-tracked in their own right.

### API

```typescript
class Arc<T> {
    static new<T>(value: T): Arc<T>;   // creates with refcount 1
    clone(): Arc<T>;                    // increments the refcount, returns a new handle
    get value(): T;                     // fatal if this handle was released
    drop(): void;                       // decrements; the last strong drop drops the inner value
    downgrade(): Weak<T>;
    asPtr(): number;                    // stable identity, for Arc::ptr_eq
    get strongCount(): number;
    [disposeSymbol](): void;            // delegates to drop()
}

class Weak<T> {
    clone(): Weak<T>;
    upgrade(): Arc<T> | null;           // null once the last strong handle dropped
    asPtr(): number;
    get weakCount(): number;
    drop(): void;
}
```

Releasing a handle ends *that handle*, even while clones live on, so every
accessor on it afterwards reports use-after-move: in Rust the moved-out binding
is no longer nameable. The last strong drop clears the inner value, so a live
`Weak` cannot keep a dropped payload reachable.

### Critical footgun

```typescript
const x = arc;        // WRONG — does NOT increment the refcount
const x = arc.clone(); // RIGHT — increments the refcount
```

---

## Mutex\<T\> / MutexGuard\<T\>

Maps `std::sync::Mutex<T>`. Trivial locking in single-threaded JS, but the Rust
API shape and ownership rules are preserved.

### API

```typescript
class Mutex<T> {
    constructor(value: T, label?: string);
    lock(): MutexGuard<T>;             // throws on re-entrant lock
    drop(): void;                      // drops the contents
}

class MutexGuard<T> extends WriteGuard<T> {}
```

Re-locking on one thread throws rather than deadlocking. Rust deadlocks here, so
this is a deliberate divergence: it reports the same bug where a hang would be
undiagnosable.

Dropping a `Mutex` while a guard on it is outstanding is fatal — impossible in
Rust, so in the port it can only mean the emitter is wrong.

### Usage

```typescript
const guard = this.state.lock();
try {
  guard.value.order.push(entry);
} finally {
  guard.drop();
}
```

---

## RwLock\<T\> / RwLockReadGuard\<T\> / RwLockWriteGuard\<T\>

Maps `std::sync::RwLock<T>`. Its own type, not an alias for `Mutex`: `read()`
and `write()` return distinct guards, and only the write guard can assign.

### API

```typescript
class RwLock<T> {
    constructor(value: T, label?: string);
    read(): RwLockReadGuard<T>;
    write(): RwLockWriteGuard<T>;
    drop(): void;                      // drops the contents
}

class RwLockReadGuard<T> extends ReadGuard<T> {
    deref(): T;
    [Symbol.iterator](): Iterator<unknown>;
}

class RwLockWriteGuard<T> extends WriteGuard<T> {}
```

---

## RefCell\<T\> / Ref\<T\> / RefMut\<T\>

Maps `std::cell::RefCell<T>`. Runtime borrow checking, with Rust's rules.

### API

```typescript
class RefCell<T> {
    constructor(value: T, options?: { onMutRelease?: () => void; label?: string });
    borrow(): Ref<T>;                  // shared access
    borrowMut(): RefMut<T>;            // exclusive access
    drop(): void;                      // drops the contents
}

class Ref<T> extends ReadGuard<T> {}
class RefMut<T> extends WriteGuard<T> {}
```

The method is `borrowMut()`, not `borrow_mut()` — it follows the port's
`snake_case` → `camelCase` naming rule like every other method.

### Borrowing rules (same as Rust)

| Current state | `borrow()` | `borrowMut()` |
|---------------|------------|-----------------|
| Not borrowed | OK | OK |
| Shared (N readers) | OK (N+1) | THROWS |
| Mut borrowed | THROWS | THROWS |

A borrow conflict throws rather than being fatal: it panics in Rust too, so the
emitted code got it right and the program got it wrong.

---

## AsyncMutex\<T\> / AsyncMutexGuard\<T\>

Maps `tokio::sync::Mutex<T>` — the lock that may be held across an `await`.

### API

```typescript
class AsyncMutex<T = undefined> {
    constructor(value?: T, label?: string);
    async acquire(): Promise<AsyncMutexGuard<T>>;
    drop(): void;                      // drops the contents; counts stranded waiters
}

class AsyncMutexGuard<T> extends WriteGuard<T> {}
```

```typescript
const guard = await this.notifyLock.acquire();
try {
  await this.notify();
} finally {
  guard.drop();
}
```

---

## Borrow\<T\> / BorrowMut\<T\>

Maps `&T` / `&mut T` in struct fields. Non-owning: dropping one would release
nothing, so they have no `drop()` at all and carry the `nonOwning` marker that
makes the cascade step over them in silence.

### API

```typescript
class Borrow<T> {
    constructor(value: T);
    get value(): T;
}

class BorrowMut<T> {
    constructor(value: T);
    get value(): T;
    set value(v: T);
}
```

---

## ThreadLocal\<T\>

Maps `thread_local!`. A static: it lives for the whole program, is never a
struct field, is never dropped, and is not leak-tracked. Marked `nonOwning`.

```typescript
class ThreadLocal<T> {
    constructor(init: T);
    with<R>(f: (value: T) => R): R;
}
```

---

## Fatal errors and the leak registry

From `drop_registry.ts`, all exported:

```typescript
class OwnershipFatal extends Error {}
function setOnFatal(handler: (message: string) => void): void;
function isPoisoned(): boolean;
function clearFatalLatch(): void;
function setCaptureStacks(enabled: boolean): void;
const disposeSymbol: typeof Symbol.dispose;
```

A fatal means the emitter is wrong — the runtime saw a state Rust rejects at
compile time. It sets a poison latch before throwing an `OwnershipFatal`, and
every liveness check reads that latch first, so a host that swallows the throw
cannot keep running over corrupted ownership. A `catch` that handles a Rust
error type must test for `OwnershipFatal` and rethrow it unconditionally.

`FinalizationRegistry` is feature-detected at load. Where it is missing the
runtime installs a no-op registry and warns once: every other check still works,
but a value that is simply forgotten goes unreported. Hermes shipped
`FinalizationRegistry` in `260318099.0.0` (2026-06-05); older Expo Go builds run
the port with leak detection off. See the Hermes note in
[../ownership.md](../ownership.md).

`setCaptureStacks(enabled)` controls allocation stacks, which cost about a
microsecond per construction. On by default except when `NODE_ENV=production`.

## `Symbol.dispose`

```typescript
export const disposeSymbol: typeof Symbol.dispose =
  (Symbol.dispose ?? Symbol.for('Symbol.dispose')) as typeof Symbol.dispose;
```

`AkObject` and `Arc` still define `[disposeSymbol]()` and it delegates to
`drop()`, but it is **not** the model and no new code should reach for it. The
cascade dispatches on `drop()`. `using` declarations are retired: Hermes refuses
to run them, so the transpiler emits explicit `.drop()` calls and `try/finally`
blocks.
