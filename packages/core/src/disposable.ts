// TS-ONLY: Maps Rust's Drop trait + RAII semantics to JS garbage collection model (see E11)
//
// Rust's Drop trait provides deterministic cleanup: when a value goes out of scope,
// drop() is called automatically. JavaScript has no equivalent — garbage collection
// is non-deterministic and provides no destructor hook.
//
// This module provides:
//   - Disposable: abstract base class for types with mandatory RAII
//   - DisposeGuard: composition-based alternative when inheritance isn't possible
//   - RefCell<T>: runtime borrow checking for scoped-mutation patterns
//
// See specs/memory-model.md for the full design rationale.

// ── Symbol.dispose polyfill ──────────────────────────────────────────────
// ES2023 introduced Symbol.dispose for the `using` declaration. Not all
// runtimes support it yet (notably Hermes for React Native). We capture
// a safe reference at module load time so the rest of the module can use
// it unconditionally.
//
// The `as typeof Symbol.dispose` cast lets TypeScript treat the fallback
// symbol identically to the native one for type-checking purposes.

export const disposeSymbol: typeof Symbol.dispose =
  (Symbol.dispose ?? Symbol.for('Symbol.dispose')) as typeof Symbol.dispose;

// ── FinalizationRegistry (diagnostic only) ───────────────────────────────
// Shared by all Disposable instances and DisposeGuard instances.
// When an object is garbage collected WITHOUT dispose() having been called,
// we log a loud error. This is purely diagnostic — it cannot perform cleanup
// because by the time the callback fires, the original object is already gone.

interface leak_info {
  label: string;
  creationStack: string;
}

const leakRegistry = new FinalizationRegistry<leak_info>((info) => {
  console.error(
    `BUG: ${info.label} was garbage collected without being disposed. ` +
    `This indicates a missing dispose() call or a missing 'using' declaration.\n` +
    `Allocated at:\n${info.creationStack}`,
  );
});

// ── Disposable base class ────────────────────────────────────────────────
//
// Abstract base class. Standard path for any type with mandatory RAII behavior.
// "Mandatory RAII" = types that have impl Drop in Rust, OR types that own
// fields with impl Drop (vicarious RAII — see specs/memory-model.md Section 10).
//
// Provides:
//   - Auto-registration with FinalizationRegistry (diagnostic on GC-without-dispose)
//   - Idempotent dispose()
//   - assertNotDisposed() guard for subclass methods
//   - [Symbol.dispose]() for `using` declaration support
//
// Usage:
//   class MySubscription extends Disposable {
//     constructor() { super('MySubscription'); }
//     protected onDispose(): void { this.inner.unsubscribe(); }
//   }

export abstract class Disposable {
  #disposed = false;
  readonly #label: string;

  constructor(label: string) {
    this.#label = label;
    const creationStack = new Error(`${label} allocated`).stack ?? '(no stack available)';
    leakRegistry.register(this, { label, creationStack }, this);
  }

  /**
   * Subclasses implement this to perform their actual cleanup.
   * Called exactly once, inside dispose().
   */
  protected abstract onDispose(): void;

  /**
   * Release resources. Idempotent: second and subsequent calls are no-ops.
   * Unregisters from the FinalizationRegistry so no false-positive leak
   * warnings are emitted.
   */
  dispose(): void {
    if (this.#disposed) return;
    this.#disposed = true;
    leakRegistry.unregister(this);
    this.onDispose();
  }

  /**
   * Guard for subclass methods. Call at the top of any method that should
   * not be used after disposal.
   *
   * @throws Error if dispose() has already been called.
   */
  protected assertNotDisposed(): void {
    if (this.#disposed) {
      throw new Error(`${this.#label} has already been disposed`);
    }
  }

  /**
   * Whether this instance has been disposed.
   */
  get isDisposed(): boolean {
    return this.#disposed;
  }

  // ES2023 `using` declaration support.
  // `using sub = node.subscribe(...)` calls [Symbol.dispose]() at block exit.
  [disposeSymbol](): void {
    this.dispose();
  }
}

// ── DisposeGuard ─────────────────────────────────────────────────────────
//
// Composition-based escape hatch for types that cannot use inheritance
// (e.g., they already extend another class).
//
// Same lifecycle semantics as Disposable but as an embeddable component.
// The host object is registered with the FinalizationRegistry; if it is
// GC'd without markDisposed(), a diagnostic error is logged.
//
// Usage:
//   class MyType {
//     private guard = new DisposeGuard(this, 'MyType');
//     dispose() { this.guard.markDisposed(); /* cleanup */ }
//     someMethod() { this.guard.assertNotDisposed(); /* work */ }
//   }

export class DisposeGuard {
  #disposed = false;
  readonly #label: string;

  /**
   * @param host — the owning object (registered with FinalizationRegistry)
   * @param label — human-readable name for diagnostics
   */
  constructor(host: object, label: string) {
    this.#label = label;
    const creationStack = new Error(`${label} allocated`).stack ?? '(no stack available)';
    leakRegistry.register(host, { label, creationStack }, host);
  }

  /**
   * Mark the host as disposed and unregister from the FinalizationRegistry.
   * Must be called from the host's dispose()/cleanup method.
   */
  markDisposed(host: object): void {
    if (this.#disposed) return;
    this.#disposed = true;
    leakRegistry.unregister(host);
  }

  /**
   * Throws if the host has been disposed.
   */
  assertNotDisposed(): void {
    if (this.#disposed) {
      throw new Error(`${this.#label} has already been disposed`);
    }
  }

  get isDisposed(): boolean {
    return this.#disposed;
  }
}

// ── RefCell<T> ───────────────────────────────────────────────────────────
//
// Enforces single-writer / multiple-reader borrowing discipline at runtime.
// Maps to Rust's RefCell<T> runtime borrow checking.
//
// In Rust, types like ResultSetWrite use RwLock/Mutex primarily for
// Drop-on-release semantics (broadcast notification when the write guard
// drops) rather than thread safety. In single-threaded JS we don't need
// locking, but we DO need:
//   1. Re-entrancy protection (no nested mutable borrows)
//   2. Guaranteed cleanup via try/finally (the "scoped mutation" pattern)
//   3. An optional onMutRelease callback (e.g., broadcast changes)
//
// Usage:
//   const cell = new RefCell(resultSet);
//   cell.withMut((rs) => { rs.add(entity); });
//   // onMutRelease fires here, e.g., broadcasting change notification
//
//   // Throws if re-entrant:
//   cell.withMut(() => { cell.withMut(() => {}); });
//   // -> Error: RefCell<...> already mutably borrowed

type BorrowState =
  | { kind: 'not_borrowed' }
  | { kind: 'shared'; count: number }
  | { kind: 'mut_borrowed' };

export class RefCell<T> {
  readonly #value: T;
  #state: BorrowState = { kind: 'not_borrowed' };
  readonly #onMutRelease: (() => void) | undefined;
  readonly #label: string;

  /**
   * @param value — the wrapped value
   * @param options.onMutRelease — called after each withMut() completes (in the finally block)
   * @param options.label — human-readable name for error messages (default: 'RefCell')
   */
  constructor(value: T, options?: { onMutRelease?: () => void; label?: string }) {
    this.#value = value;
    this.#onMutRelease = options?.onMutRelease;
    this.#label = options?.label ?? 'RefCell';
  }

  /**
   * Exclusive mutable access. Throws if any borrow (shared or mutable) is active.
   * Runs `fn` in try/finally; the onMutRelease callback fires in the finally block
   * after fn completes (whether it throws or not).
   */
  withMut<R>(fn: (value: T) => R): R {
    if (this.#state.kind !== 'not_borrowed') {
      if (this.#state.kind === 'mut_borrowed') {
        throw new Error(`${this.#label} already mutably borrowed`);
      }
      throw new Error(`${this.#label} already shared-borrowed (count: ${this.#state.count})`);
    }
    this.#state = { kind: 'mut_borrowed' };
    try {
      return fn(this.#value);
    } finally {
      this.#state = { kind: 'not_borrowed' };
      this.#onMutRelease?.();
    }
  }

  /**
   * Shared read-only access. Throws if a mutable borrow is active.
   * Multiple shared borrows can be active simultaneously.
   */
  withRef<R>(fn: (value: T) => R): R {
    if (this.#state.kind === 'mut_borrowed') {
      throw new Error(`${this.#label} already mutably borrowed — cannot take shared borrow`);
    }
    if (this.#state.kind === 'shared') {
      this.#state = { kind: 'shared', count: this.#state.count + 1 };
    } else {
      this.#state = { kind: 'shared', count: 1 };
    }
    try {
      return fn(this.#value);
    } finally {
      if (this.#state.kind === 'shared') {
        if (this.#state.count <= 1) {
          this.#state = { kind: 'not_borrowed' };
        } else {
          this.#state = { kind: 'shared', count: this.#state.count - 1 };
        }
      }
    }
  }
}
