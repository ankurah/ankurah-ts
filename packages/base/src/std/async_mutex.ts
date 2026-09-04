// TS-ONLY: Maps Rust's tokio::sync::Mutex to JS async serialization

import { DropGuard } from './drop.ts';
import { WriteGuard, dropContainer } from './guard.ts';
import type { Slot } from '../object.ts';

/**
 * Held while a critical section runs; releases the mutex when dropped.
 *
 * A guard rather than a bare release closure, because a critical section that
 * throws must still release: an emitted `finally` drops the guard, where it
 * would simply never have called the closure.
 */
export class AsyncMutexGuard<T> extends WriteGuard<T> {
  /** @internal */
  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(slot, release, `AsyncMutexGuard on ${label}`);
  }
}

/**
 * Async mutex for serializing operations across await points.
 * 1:1 equivalent of Rust's tokio::sync::Mutex<T>, which most of the port uses as
 * tokio::sync::Mutex<()> — hence the default, so `new AsyncMutex()` still reads
 * the same. It is an owned value with drop glue in Rust, so it is tracked and
 * dropped exactly like the synchronous Mutex.
 */
export class AsyncMutex<T = undefined> {
  #value: T;
  #held = false;
  #waiting = 0;
  #queue: Promise<void> = Promise.resolve();
  readonly #guard: DropGuard;
  readonly #label: string;

  constructor(value: T = undefined as T, label?: string) {
    this.#value = value;
    this.#label = label ?? 'AsyncMutex';
    this.#guard = new DropGuard(this, this.#label);
  }

  async acquire(): Promise<AsyncMutexGuard<T>> {
    this.#guard.assertNotDropped();
    let release!: () => void;
    const next = new Promise<void>((resolve) => {
      release = resolve;
    });
    const prev = this.#queue;
    this.#queue = next;
    // A caller parked on prev is as much an outstanding claim on this mutex as
    // one already holding it: dropping out from under it would strand it here
    // forever, waiting for a turn that can no longer come.
    this.#waiting++;
    try {
      await prev;
    } finally {
      this.#waiting--;
    }
    // The mutex may have been dropped while this call waited its turn.
    this.#guard.assertNotDropped();
    this.#held = true;
    const slot: Slot<T> = {
      get: () => this.#value,
      set: (v) => { this.#value = v; },
    };
    return new AsyncMutexGuard<T>(slot, () => {
      this.#held = false;
      release();
    }, this.#label);
  }

  /**
   * Dropping a tokio Mutex<T> drops the T inside it, and a guard still holding
   * the lock means the emitted drop scope is wrong — the same contract as the
   * synchronous Mutex.
   */
  drop(): void {
    dropContainer(
      this,
      this.#guard,
      this.#label,
      () => {
        if (this.#held) return 'AsyncMutexGuard';
        if (this.#waiting > 0) return 'queued acquire()';
        return null;
      },
      () => this.#value,
    );
  }
}
