// TS-ONLY: Maps Rust's thread_local! macro to a module-level container.
// JS is single-threaded, so thread_local is just a wrapper providing .with() access.

import { nonOwning } from '../object.ts';

export class ThreadLocal<T> {
  // A thread_local! is a static: it lives for the whole program and is never a
  // struct field, so no cascade legitimately reaches one. Marked nonOwning so
  // that if a cascade ever does reach one, it steps over it in silence rather
  // than reporting an unwrapped foreign object.
  readonly [nonOwning] = true;
  private value: T;

  constructor(init: T) {
    this.value = init;
  }

  /** Access the thread-local value via callback (mirrors Rust's .with() API) */
  with<R>(f: (value: T) => R): R {
    return f(this.value);
  }
}
