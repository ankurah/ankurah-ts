# Memory Model: Provided Types

**API documentation for the std-equivalent utility types.** These types implement the Rust ownership patterns described in [overview.md](overview.md). Source location: `packages/core/src/disposable.ts` (to be moved to a dedicated `std/` or `ownership/` folder).

---

## Disposable

Base class for types requiring explicit cleanup. Equivalent to Rust types with `impl Drop` or vicarious RAII.

### API

```typescript
abstract class Disposable {
    constructor(label: string);           // registers with FinalizationRegistry
    protected abstract onDispose(): void; // implement cleanup here
    dispose(): void;                      // idempotent; calls onDispose() once
    protected assertNotDisposed(): void;  // throws if already disposed
    get isDisposed(): boolean;
    [Symbol.dispose](): void;             // delegates to dispose()
}
```

### Error behavior

- `dispose()` is idempotent — calling it twice is a no-op (second call returns immediately).
- If `onDispose()` throws, the object is still marked as disposed and unregistered from FinalizationRegistry. The error propagates to the caller. This means partial cleanup is possible — `onDispose()` implementations that dispose multiple owned fields should use try/finally to ensure all fields are disposed even if an earlier one throws.

### Usage

```typescript
class MySubscription extends Disposable {
    #inner: SubscriptionHandle;

    constructor(handle: SubscriptionHandle) {
        super('MySubscription');
        this.#inner = handle;
    }

    protected onDispose(): void {
        this.#inner.unsubscribe();
    }

    getValue(): string {
        this.assertNotDisposed();
        return this.#inner.currentValue();
    }
}

// With `using`:
{ using sub = createSubscription(); }

// Manual:
const sub = createSubscription();
try { /* ... */ } finally { sub.dispose(); }
```

---

## DisposeGuard

Composition helper for types that need disposal semantics but already extend another class.

### API

```typescript
class DisposeGuard {
    constructor(host: object, label: string);
    markDisposed(host: object): void;  // call from host's cleanup method
    assertNotDisposed(): void;          // throws if disposed
    get isDisposed(): boolean;
}
```

### Usage

```typescript
class MyWidget extends SomeFrameworkBase {
    #guard = new DisposeGuard(this, 'MyWidget');
    #subscription: ReactorSubscription;

    dispose(): void {
        this.#guard.markDisposed(this);
        this.#subscription.dispose();
    }

    render(): void {
        this.#guard.assertNotDisposed();
        // ...
    }
}
```

---

## Mutex\<T\>

1:1 equivalent of Rust's `std::sync::Mutex<T>`. In JS the locking is trivial (single-threaded), but the type preserves the Rust API shape so translated code reads the same.

### API

```typescript
class Mutex<T> {
    constructor(value: T);
    lock(): MutexGuard<T>;   // returns a Disposable guard — use with `using`
}

class MutexGuard<T> extends Disposable {
    get value(): T;           // access the guarded value
    set value(v: T);
    // onDispose() can fire side-effects (e.g., broadcast on drop)
}
```

### Usage

```typescript
// Rust: let guard = self.state.lock().unwrap(); ... guard drops
// TS:
{
    using guard = this.state.lock();
    guard.value.order.push(entry);
} // guard disposed -> onDispose() fires
```

The guard's `onDispose()` is where Drop side-effects go (broadcast, notification, etc.). The `Mutex` itself just provides the API shape and re-entrancy protection (throws if `lock()` is called while a guard is active).

---

## RefCell\<T\>

1:1 equivalent of Rust's `std::cell::RefCell<T>`. Runtime borrow checking — panics on double mutable borrow, just like Rust.

### API

```typescript
class RefCell<T> {
    constructor(value: T);
    borrow(): Ref<T>;           // shared access — use with `using`
    borrow_mut(): RefMut<T>;    // exclusive access — use with `using`
}

class Ref<T> extends Disposable {
    get value(): T;
}

class RefMut<T> extends Disposable {
    get value(): T;
    set value(v: T);
}
```

### Borrowing Rules (same as Rust)

| Current state | `borrow()` | `borrow_mut()` |
|---------------|------------|-----------------|
| Not borrowed | OK | OK |
| Shared (N readers) | OK (N+1) | THROWS |
| Mut borrowed | THROWS | THROWS |

### Usage

```typescript
// Rust: let mut guard = self.inner.borrow_mut(); guard.field = value;
// TS:
{
    using guard = this.inner.borrow_mut();
    guard.value.field = value;
} // borrow released on dispose
```

---

## PromiseMutex

Async serialization primitive. Equivalent to Rust's `tokio::sync::Mutex<()>` — serializes async operations that must not interleave.

### API

```typescript
class PromiseMutex {
    async run<T>(fn: () => Promise<T>): Promise<T>;
}
```

### Usage

```typescript
const lock = new PromiseMutex();

// These calls are serialized — the second waits for the first to complete:
await lock.run(async () => { /* critical section 1 */ });
await lock.run(async () => { /* critical section 2 */ });
```

### Implementation

```typescript
class PromiseMutex {
    #chain: Promise<void> = Promise.resolve();

    async run<T>(fn: () => Promise<T>): Promise<T> {
        const prev = this.#chain;
        let resolve: () => void;
        this.#chain = new Promise<void>((r) => { resolve = r; });
        await prev;
        try {
            return await fn();
        } finally {
            resolve!();
        }
    }
}
```

---

## Symbol.dispose Polyfill

### Setup

```typescript
export const disposeSymbol: typeof Symbol.dispose =
  (Symbol.dispose ?? Symbol.for('Symbol.dispose')) as typeof Symbol.dispose;
```

Safe on all runtimes:
- **V8 (Node, Chrome, Bun)**: `Symbol.dispose` exists natively
- **Hermes (React Native)**: Falls back to `Symbol.for('Symbol.dispose')`
- **JavaScriptCore (Safari)**: Fallback covers older versions

### Babel plugin for React Native / Expo

The `using` syntax requires transpilation on runtimes that don't support it natively:

```json
{ "plugins": ["@babel/plugin-transform-explicit-resource-management"] }
```

Metro handles this transform with the above plugin. Platform support for `using`/`dispose` via this transpilation is the key delineator — `using`/`dispose` + guards is the preferred approach provided this transform works across all target platforms (browsers, RN/Expo, etc.).

### Timing

The polyfill must be loaded before any class that uses `[disposeSymbol]()`. Guaranteed by the module-level `const` in the source file — just ensure it's imported before any Disposable subclass is instantiated.
