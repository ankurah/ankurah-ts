// TS-ONLY: Maps Rust's Drop trait + RAII semantics to JS garbage collection model (see E11)
//
// Rust's Drop trait provides deterministic cleanup: when a value goes out of scope,
// drop() is called automatically. JavaScript has no equivalent — garbage collection
// is non-deterministic and provides no destructor hook.
//
// This module provides:
//   - Disposable: abstract base class for types with mandatory RAII (maps to impl Drop)
//   - DisposeGuard: composition-based alternative when inheritance isn't possible
//   - disposeSymbol: polyfill for Symbol.dispose
//
// See port/ownership.md for the full design rationale.

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
// the severity determines the response:
//   - 'fatal': queueMicrotask(() => { throw ... }) — crashes hard
//   - 'warning': console.error — diagnostic only

interface leak_info {
  label: string;
  creationStack: string;
  severity: 'fatal' | 'warning';
}

export const leakRegistry = new FinalizationRegistry<leak_info>((info) => {
  const message =
    `BUG: ${info.label} was garbage collected without being disposed. ` +
    `This indicates a missing dispose() call or a missing 'using' declaration.\n` +
    `Allocated at:\n${info.creationStack}`;

  if (info.severity === 'fatal') {
    queueMicrotask(() => {
      throw new Error(message);
    });
  } else {
    console.error(message);
  }
});

// ── Disposable base class ────────────────────────────────────────────────
//
// Abstract base class. Standard path for any type with mandatory RAII behavior.
// "Mandatory RAII" = types that have impl Drop in Rust, OR types that own
// fields with impl Drop (vicarious RAII — see port/ownership.md).
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

  /**
   * @param label — human-readable name for diagnostics
   * @param severity — 'fatal' crashes on leak (correctness-critical),
   *                    'warning' logs on leak (resource hygiene). Default: 'warning'.
   */
  constructor(label: string, severity: 'fatal' | 'warning' = 'warning') {
    this.#label = label;
    const creationStack = new Error(`${label} allocated`).stack ?? '(no stack available)';
    leakRegistry.register(this, { label, creationStack, severity }, this);
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
   * @param severity — 'fatal' or 'warning' (default: 'warning')
   */
  constructor(host: object, label: string, severity: 'fatal' | 'warning' = 'warning') {
    this.#label = label;
    const creationStack = new Error(`${label} allocated`).stack ?? '(no stack available)';
    leakRegistry.register(host, { label, creationStack, severity }, host);
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
