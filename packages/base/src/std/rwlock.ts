// TS-ONLY: Maps Rust's std::sync::RwLock<T> to JS
//
// In JS the locking is trivial (single-threaded), but the type preserves
// the Rust API shape so translated code reads the same.

import { DropGuard } from './drop.ts';
import { ReadGuard, WriteGuard, dropContainer } from './guard.ts';
import type { Slot } from '../object.ts';

export class RwLock<T> {
  #value: T;
  #readers = 0;
  #writing = false;
  readonly #guard: DropGuard;
  readonly #label: string;

  constructor(value: T, label?: string) {
    this.#value = value;
    this.#label = label ?? 'RwLock';
    this.#guard = new DropGuard(this, this.#label);
  }

  #slot(): Slot<T> {
    return {
      get: () => this.#value,
      set: (v) => { this.#value = v; },
    };
  }

  read(): RwLockReadGuard<T> {
    this.#guard.assertNotDropped();
    if (this.#writing) {
      throw new Error('RwLock: cannot read — write lock held');
    }
    this.#readers++;
    return new RwLockReadGuard<T>(this.#slot(), () => { this.#readers--; }, this.#label);
  }

  write(): RwLockWriteGuard<T> {
    this.#guard.assertNotDropped();
    if (this.#writing) {
      throw new Error('RwLock: cannot write — write lock held');
    }
    if (this.#readers > 0) {
      throw new Error('RwLock: cannot write — read locks held');
    }
    this.#writing = true;
    return new RwLockWriteGuard<T>(this.#slot(), () => { this.#writing = false; }, this.#label);
  }

  /**
   * Dropping a RwLock<T> in Rust drops the T inside it. The value sits in a
   * #private field the owning object's cascade cannot see, so the RwLock drops
   * it. A guard still holding a lock means the emitted drop scope is wrong, and
   * releasing the value under it would be the corruption Rust prevents.
   */
  drop(): void {
    dropContainer(
      this,
      this.#guard,
      this.#label,
      () => {
        if (this.#writing) return 'RwLockWriteGuard';
        if (this.#readers > 0) return 'RwLockReadGuard';
        return null;
      },
      () => this.#value,
    );
  }
}

export class RwLockReadGuard<T> extends ReadGuard<T> {
  /** @internal */
  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(slot, release, `RwLockReadGuard on ${label}`);
  }

  /** Deref — access inner value directly */
  deref(): T {
    return this.value;
  }

  /** Iterate the inner value (if iterable) */
  [Symbol.iterator](): Iterator<any> {
    const v = this.value as any;
    if (v && typeof v[Symbol.iterator] === 'function') {
      return v[Symbol.iterator]();
    }
    throw new Error('RwLockReadGuard: inner value is not iterable');
  }
}

export class RwLockWriteGuard<T> extends WriteGuard<T> {
  /** @internal */
  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(slot, release, `RwLockWriteGuard on ${label}`);
  }
}
