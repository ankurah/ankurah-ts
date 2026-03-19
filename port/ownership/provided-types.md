# Provided Types — API Reference

**Source location**: `packages/base/src/`

These types implement the ownership patterns described in [ownership.md](../ownership.md). Every ported Rust type extends one of these base classes.

---

## AkObject

Base class for all ported Rust types. Provides automatic cascade disposal and leak detection.

### API

```typescript
class AkObject {
    constructor();                    // registers with FinalizationRegistry (fatal on leak)
    drop(): void;                     // override in Drop subclass for custom cleanup
    [Symbol.dispose](): void;         // idempotent drop glue: calls drop(), cascades to owned fields
    protected assertNotDropped(): void;
    get isDropped(): boolean;
}
```

### Cascade behavior

`[Symbol.dispose]()` walks all own properties and calls `[Symbol.dispose]()` on each. Also recurses into arrays of disposable items. This means a parent automatically disposes all its owned children.

Use `Borrow<T>` / `BorrowMut<T>` to mark fields that should NOT be cascaded (non-owning references).

---

## Struct

Base class for ported Rust structs. Empty subclass of AkObject.

```typescript
class Struct extends AkObject {}
```

Usage: `struct Foo` → `class Foo extends Struct`.

---

## Enum\<V\>

Base class for ported Rust enums. Extends AkObject with typed variant support.

### API

```typescript
class Enum<V extends Record<string, object>> extends AkObject {
    readonly type: string & keyof V;
    readonly value: V[keyof V];
    constructor(type: string & keyof V, value: V[keyof V]);
    match<R>(arms: { [K in keyof V]: (value: V[K]) => R }): R;
    is<K extends keyof V>(variant: K): boolean;
    toString(): string;
}
```

Enum overrides cascade to also walk `this.value`'s properties (variant data fields), including arrays.

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

## Drop

Abstract subclass of AkObject for types with `impl Drop` in Rust. Forces you to implement `drop()`.

### API

```typescript
abstract class Drop extends AkObject {
    abstract override drop(): void;   // implement cleanup here
}
```

### Usage

```typescript
class Transaction extends Drop {
    #committed = false;

    drop(): void {
        if (!this.#committed) {
            this.rollback();
        }
    }
}

// With `using`:
{ using trx = node.beginTransaction(); /* ... */ }
// trx.drop() called at block exit
```

---

## DropGuard

Composition helper for types that need leak detection but already extend another class.

### API

```typescript
class DropGuard {
    constructor(host: object);        // registers host with FinalizationRegistry (fatal on leak)
    markDropped(host: object): void;  // call from host's cleanup method
    assertNotDropped(): void;
    get isDropped(): boolean;
}
```

---

## Arc\<T\> / Weak\<T\>

Refcounted shared ownership. Arc has its own leak detection (independent of AkObject).

### API

```typescript
class Arc<T> {
    static new<T>(value: T): Arc<T>;  // creates with refcount 1
    clone(): Arc<T>;                   // increments refcount, returns new Arc
    get value(): T;                    // access inner value (throws if dropped)
    drop(): void;                      // decrements refcount; drops inner at 0
    downgrade(): Weak<T>;              // create weak reference
    get strongCount(): number;
    [Symbol.dispose](): void;          // delegates to drop()
}

class Weak<T> {
    upgrade(): Arc<T> | null;          // returns null if inner dropped
    drop(): void;                      // decrements weak count
}
```

### Critical footgun

```typescript
const x = arc;        // WRONG — does NOT increment refcount
const x = arc.clone(); // RIGHT — increments refcount
```

---

## Mutex\<T\> / MutexGuard\<T\>

Maps `std::sync::Mutex<T>`. Trivial locking in single-threaded JS, but preserves Rust API shape.

### API

```typescript
class Mutex<T> {
    constructor(value: T);
    lock(): MutexGuard<T>;             // throws if already locked (re-entrancy)
}

class MutexGuard<T> extends Drop {
    get value(): T;
    set value(v: T);
    drop(): void;                      // releases lock
}
```

### Usage

```typescript
{
    using guard = this.state.lock();
    guard.value.order.push(entry);
} // guard disposed → lock released
```

---

## RefCell\<T\> / Ref\<T\> / RefMut\<T\>

Maps `std::cell::RefCell<T>`. Runtime borrow checking — throws on double mutable borrow.

### API

```typescript
class RefCell<T> {
    constructor(value: T, options?: { onMutRelease?: () => void; label?: string });
    borrow(): Ref<T>;                  // shared access — use with `using`
    borrow_mut(): RefMut<T>;           // exclusive access — use with `using`
}

class Ref<T> extends Drop {
    get value(): T;
    drop(): void;                     // releases shared borrow
}

class RefMut<T> extends Drop {
    get value(): T;
    set value(v: T);
    drop(): void;                     // releases exclusive borrow
}
```

### Borrowing rules (same as Rust)

| Current state | `borrow()` | `borrow_mut()` |
|---------------|------------|-----------------|
| Not borrowed | OK | OK |
| Shared (N readers) | OK (N+1) | THROWS |
| Mut borrowed | THROWS | THROWS |

---

## AsyncMutex

Async serialization. Maps `tokio::sync::Mutex<()>`.

### API

```typescript
class AsyncMutex {
    async run<T>(fn: () => Promise<T>): Promise<T>;  // serializes async operations
}
```

---

## Borrow\<T\> / BorrowMut\<T\>

Maps `&T` / `&mut T` in struct fields. Marks a field as non-owning so cascade skips it.

### API

```typescript
class Borrow<T> {
    constructor(value: T);
    get value(): T;
    [Symbol.dispose](): void;          // no-op — prevents cascade
}

class BorrowMut<T> {
    constructor(value: T);
    get value(): T;
    set value(v: T);
    [Symbol.dispose](): void;          // no-op — prevents cascade
}
```

---

## Symbol.dispose Polyfill

```typescript
export const disposeSymbol: typeof Symbol.dispose =
  (Symbol.dispose ?? Symbol.for('Symbol.dispose')) as typeof Symbol.dispose;
```

Safe on all runtimes. The `using` syntax requires Babel plugin `@babel/plugin-transform-explicit-resource-management` on runtimes that don't support it natively (Hermes, older Safari).

The polyfill must be loaded before any class that uses `[disposeSymbol]()`.
