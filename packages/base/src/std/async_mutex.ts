// TS-ONLY: Maps Rust's tokio::sync::Mutex to JS async serialization

import { DropGuard } from './drop.ts';
import { WriteGuard, dropContainer } from './guard.ts';
import { Result } from '../result.ts';
import { fatalOutstandingGuard } from '../drop_registry.ts';
import { TryLockError } from '../tokio/lock_error.ts';
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
 *
 * `acquire()` is this port's name for taking the lock, and `lock()` is tokio's;
 * both are here and they are the same call, so emitted code can spell it the
 * way the Rust source does.
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
    return this.#grant(release);
  }

  /** tokio's name for acquire(). The same call. */
  lock(): Promise<AsyncMutexGuard<T>> {
    this.#guard.assertNotDropped();
    return this.acquire();
  }

  /** Take the lock only if it can be had without waiting. */
  try_lock(): Result<AsyncMutexGuard<T>, TryLockError> {
    this.#guard.assertNotDropped();
    if (this.#held || this.#waiting > 0) return Result.Err(new TryLockError());
    let release!: () => void;
    // Nothing holds the lock and nobody is queued, so the promise at the tail
    // of the queue has already resolved: chaining onto it here is what makes a
    // later acquire() wait for the guard this hands back.
    this.#queue = new Promise<void>((resolve) => {
      release = resolve;
    });
    return Result.Ok(this.#grant(release));
  }

  /**
   * `into_inner(self)` — take the value and consume the mutex. The value moves
   * to the caller, so nothing is released here, and the mutex leaves the leak
   * registry rather than being dropped. Using it again reports use-after-drop:
   * this is a container, not an AkObject, so it has no separate moved state.
   */
  into_inner(): T {
    this.#guard.assertNotDropped();
    this.#refuseWhileBorrowed();
    const value = this.#value;
    this.#guard.markDropped(this);
    return value;
  }

  /**
   * Rust's `get_mut(&mut self) -> &mut T`: exclusive access with no locking,
   * because `&mut self` already proves nobody else holds the lock.
   *
   * DELIBERATE LIMITATION: this hands back the value, not the place it sits in.
   * Rust's `*mutex.get_mut() = v` — which replaces what the mutex holds and
   * drops what was there — has no equivalent through this; a guard is where
   * that lives.
   */
  get_mut(): T {
    this.#guard.assertNotDropped();
    this.#refuseWhileBorrowed();
    return this.#value;
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
      () => this.#outstanding(),
      () => this.#value,
    );
  }

  #grant(release: () => void): AsyncMutexGuard<T> {
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

  /** What `&mut self` and `self` both assume: no guard, and nobody queued. */
  #refuseWhileBorrowed(): void {
    const held = this.#outstanding();
    if (held !== null) fatalOutstandingGuard(this.#label, held);
  }

  #outstanding(): string | null {
    if (this.#held) return 'AsyncMutexGuard';
    if (this.#waiting > 0) return 'queued acquire()';
    return null;
  }
}
