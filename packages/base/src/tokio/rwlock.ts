// TS-ONLY: Maps tokio::sync::RwLock<T> to JS async serialization.
//
// The synchronous RwLock in std/rwlock.ts hands its guard back immediately and
// throws on a conflict, because in Rust a conflict there is a deadlock. This
// one waits instead, which is what `collectionset.rs` needs: it reads the
// collection map on the common path and takes the write lock only to install a
// collection it had to open first, across an await.
//
// The queue is first-come-first-served, as tokio's is: a waiting writer blocks
// readers that arrive after it, so a steady stream of reads cannot starve it.

import { DropGuard } from '../std/drop.ts';
import { ReadGuard, WriteGuard, dropContainer } from '../std/guard.ts';
import { Result } from '../result.ts';
import { fatalOutstandingGuard } from '../drop_registry.ts';
import { TryLockError } from './lock_error.ts';
import type { Slot } from '../object.ts';

export { TryLockError };

interface Waiter {
  readonly write: boolean;
  readonly grant: () => void;
}

/** Async reader/writer lock — the 1:1 equivalent of tokio::sync::RwLock<T>. */
export class AsyncRwLock<T> {
  #value: T;
  #readers = 0;
  #writing = false;
  readonly #queue: Waiter[] = [];
  readonly #guard: DropGuard;
  readonly #label: string;

  constructor(value: T, label?: string) {
    this.#value = value;
    this.#label = label ?? 'AsyncRwLock';
    this.#guard = new DropGuard(this, this.#label);
  }

  /** Take a shared read lock, waiting for any writer ahead of it. */
  async read(): Promise<AsyncRwLockReadGuard<T>> {
    this.#guard.assertNotDropped();
    await this.#enqueue(false);
    // The lock may have been dropped while this call waited its turn.
    this.#guard.assertNotDropped();
    return new AsyncRwLockReadGuard<T>(this.#slot(), () => this.#releaseRead(), this.#label);
  }

  /** Take the exclusive write lock, waiting for the readers and writers ahead of it. */
  async write(): Promise<AsyncRwLockWriteGuard<T>> {
    this.#guard.assertNotDropped();
    await this.#enqueue(true);
    this.#guard.assertNotDropped();
    return new AsyncRwLockWriteGuard<T>(this.#slot(), () => this.#releaseWrite(), this.#label);
  }

  /**
   * Read only if it can be had without waiting. A writer already in the queue
   * makes this fail even when nobody holds the lock, so that a burst of reads
   * cannot jump ahead of it.
   */
  try_read(): Result<AsyncRwLockReadGuard<T>, TryLockError> {
    this.#guard.assertNotDropped();
    if (this.#writing || this.#queue.length > 0) return Result.Err(new TryLockError());
    this.#readers++;
    return Result.Ok(new AsyncRwLockReadGuard<T>(this.#slot(), () => this.#releaseRead(), this.#label));
  }

  /** Take the write lock only if it can be had without waiting. */
  try_write(): Result<AsyncRwLockWriteGuard<T>, TryLockError> {
    this.#guard.assertNotDropped();
    if (this.#writing || this.#readers > 0 || this.#queue.length > 0) return Result.Err(new TryLockError());
    this.#writing = true;
    return Result.Ok(new AsyncRwLockWriteGuard<T>(this.#slot(), () => this.#releaseWrite(), this.#label));
  }

  /**
   * `into_inner(self)` — take the value and consume the lock. The value moves
   * to the caller, so nothing is released here, and the lock leaves the leak
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
   * Rust's `*lock.get_mut() = v` — which replaces what the lock holds and drops
   * what was there — has no equivalent through this; a write guard is where
   * that lives.
   */
  get_mut(): T {
    this.#guard.assertNotDropped();
    this.#refuseWhileBorrowed();
    return this.#value;
  }

  /**
   * Dropping a RwLock<T> in Rust drops the T inside it. A guard still holding
   * the lock, or a call still queued for it, means the emitted drop scope is
   * wrong: the guard would read released memory, and the queued caller would
   * wait for a turn that can no longer come.
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

  /** What `&mut self` and `self` both assume: no guard, and nobody queued. */
  #refuseWhileBorrowed(): void {
    const held = this.#outstanding();
    if (held !== null) fatalOutstandingGuard(this.#label, held);
  }

  #outstanding(): string | null {
    if (this.#writing) return 'AsyncRwLockWriteGuard';
    if (this.#readers > 0) return 'AsyncRwLockReadGuard';
    if (this.#queue.length > 0) return 'queued read()/write()';
    return null;
  }

  #slot(): Slot<T> {
    return {
      get: () => this.#value,
      set: (v) => { this.#value = v; },
    };
  }

  #enqueue(write: boolean): Promise<void> {
    return new Promise<void>((grant) => {
      this.#queue.push({ write, grant });
      this.#grantFromFront();
    });
  }

  /**
   * Hand the lock to whoever has waited longest, and keep going while the next
   * one is compatible — which lets a run of queued readers in together.
   */
  #grantFromFront(): void {
    while (this.#queue.length > 0) {
      const head = this.#queue[0] as Waiter;
      if (head.write) {
        if (this.#writing || this.#readers > 0) return;
        this.#queue.shift();
        this.#writing = true;
      } else {
        if (this.#writing) return;
        this.#queue.shift();
        this.#readers++;
      }
      head.grant();
    }
  }

  #releaseRead(): void {
    this.#readers--;
    this.#grantFromFront();
  }

  #releaseWrite(): void {
    this.#writing = false;
    this.#grantFromFront();
  }
}

export class AsyncRwLockReadGuard<T> extends ReadGuard<T> {
  /** @internal */
  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(slot, release, `AsyncRwLockReadGuard on ${label}`);
  }

  /** Deref — access inner value directly */
  deref(): T {
    return this.value;
  }
}

export class AsyncRwLockWriteGuard<T> extends WriteGuard<T> {
  /** @internal */
  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(slot, release, `AsyncRwLockWriteGuard on ${label}`);
  }
}
