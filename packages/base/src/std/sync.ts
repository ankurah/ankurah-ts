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

import { DropGuard } from './drop.ts';
import { WriteGuard, dropContainer } from './guard.ts';
import type { Slot } from '../object.ts';

// ── Mutex<T> ─────────────────────────────────────────────────────────────
//
// Usage:
//   const m = new Mutex(initialValue);
//   const guard = m.lock();
//   guard.value.field = 42;
//   guard.drop();

export class Mutex<T> {
  #value: T;
  #locked = false;
  readonly #guard: DropGuard;
  readonly #label: string;

  constructor(value: T, label?: string) {
    this.#value = value;
    this.#label = label ?? 'Mutex';
    this.#guard = new DropGuard(this, this.#label);
  }

  /**
   * Acquire the lock and return a MutexGuard. Re-locking on one thread
   * deadlocks in Rust — it hangs rather than failing — so this throws instead:
   * the same bug, reported where a deadlock would be undiagnosable.
   */
  lock(): MutexGuard<T> {
    this.#guard.assertNotDropped();
    if (this.#locked) {
      throw new Error(`${this.#label} already locked`);
    }
    this.#locked = true;
    // The guard is handed this Mutex's own storage, not a copy of the value, so
    // an assignment through the guard lands here the way *guard = v does in Rust.
    const slot: Slot<T> = {
      get: () => this.#value,
      set: (v) => { this.#value = v; },
    };
    return new MutexGuard<T>(slot, () => { this.#locked = false; }, this.#label);
  }

  /**
   * Dropping a Mutex<T> in Rust drops the T inside it. The value sits in a
   * #private field the owning object's cascade cannot see, so the Mutex drops
   * it. A guard still holding the lock means the emitted drop scope is wrong,
   * and releasing the value under it would be the corruption Rust prevents.
   */
  drop(): void {
    dropContainer(
      this,
      this.#guard,
      this.#label,
      () => (this.#locked ? 'MutexGuard' : null),
      () => this.#value,
    );
  }
}

// ── MutexGuard<T> ────────────────────────────────────────────────────────

/** Guard returned by Mutex.lock(). Releases the lock when dropped. */
export class MutexGuard<T> extends WriteGuard<T> {
  /** @internal */
  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(slot, release, `MutexGuard on ${label}`);
  }
}
