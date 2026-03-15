// TS-ONLY: Maps Rust's std::sync module to JS (see E11)
//
// Provides:
//   - Mutex<T>: synchronous mutex (maps to std::sync::Mutex<T>)
//   - MutexGuard<T>: lock guard (maps to std::sync::MutexGuard<T>)
//
// In JS the locking is trivial (single-threaded), but the type preserves
// the Rust API shape so translated code reads the same.
//
// See port/ownership.md and port/ownership/provided-types.md for API spec.

import { Drop } from './drop.ts';

// ── Mutex<T> ─────────────────────────────────────────────────────────────
//
// Usage:
//   const m = new Mutex(initialValue);
//   { using guard = m.lock(); guard.value.field = 42; }

export class Mutex<T> {
  #value: T;
  #locked = false;
  readonly #label: string;

  constructor(value: T, label?: string) {
    this.#value = value;
    this.#label = label ?? 'Mutex';
  }

  /**
   * Acquire the lock and return a MutexGuard. Throws if already locked (re-entrancy).
   */
  lock(): MutexGuard<T> {
    if (this.#locked) {
      throw new Error(`${this.#label} already locked`);
    }
    this.#locked = true;
    return new MutexGuard<T>(this.#value, () => {
      this.#locked = false;
    }, this.#label);
  }
}

// ── MutexGuard<T> ────────────────────────────────────────────────────────

/**
 * Guard returned by Mutex.lock(). Provides access to the guarded value.
 * On drop, releases the lock and fires any drop side-effects.
 */
export class MutexGuard<T> extends Drop {
  #value: T;
  readonly #release: () => void;

  /** @internal */
  constructor(value: T, release: () => void, label: string) {
    super();
    this.#value = value;
    this.#release = release;
  }

  get value(): T {
    this.assertNotDropped();
    return this.#value;
  }

  set value(v: T) {
    this.assertNotDropped();
    this.#value = v;
  }

  drop(): void {
    this.#release();
  }
}
