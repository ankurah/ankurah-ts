// TS-ONLY: Maps Rust's std::sync::RwLock<T> to JS
//
// In JS the locking is trivial (single-threaded), but the type preserves
// the Rust API shape so translated code reads the same.

import { Drop } from './drop.ts';

export class RwLock<T> {
  #value: T;
  #readers = 0;
  #writing = false;

  constructor(value: T) {
    this.#value = value;
  }

  static new<T>(value: T): RwLock<T> {
    return new RwLock(value);
  }

  read(): RwLockReadGuard<T> {
    if (this.#writing) {
      throw new Error('RwLock: cannot read — write lock held');
    }
    this.#readers++;
    return new RwLockReadGuard<T>(this.#value, () => {
      this.#readers--;
    });
  }

  write(): RwLockWriteGuard<T> {
    if (this.#writing) {
      throw new Error('RwLock: cannot write — write lock held');
    }
    if (this.#readers > 0) {
      throw new Error('RwLock: cannot write — read locks held');
    }
    this.#writing = true;
    return new RwLockWriteGuard<T>(this.#value, (newValue) => {
      this.#value = newValue;
      this.#writing = false;
    });
  }
}

export class RwLockReadGuard<T> extends Drop {
  readonly #value: T;
  readonly #release: () => void;

  constructor(value: T, release: () => void) {
    super();
    this.#value = value;
    this.#release = release;
  }

  get value(): T {
    this.assertNotDropped();
    return this.#value;
  }

  /** Deref — access inner value directly */
  deref(): T {
    return this.value;
  }

  /** Iterate the inner value (if iterable) */
  [Symbol.iterator](): Iterator<any> {
    const v = this.value;
    if (v && typeof (v as any)[Symbol.iterator] === 'function') {
      return (v as any)[Symbol.iterator]();
    }
    throw new Error('RwLockReadGuard: inner value is not iterable');
  }

  drop(): void {
    this.#release();
  }
}

export class RwLockWriteGuard<T> extends Drop {
  #value: T;
  readonly #release: (value: T) => void;

  constructor(value: T, release: (value: T) => void) {
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
    this.#release(this.#value);
  }
}
