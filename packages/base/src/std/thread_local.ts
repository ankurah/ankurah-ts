// TS-ONLY: Maps Rust's thread_local! macro to a module-level container.
// JS is single-threaded, so thread_local is just a wrapper providing .with() access.

export class ThreadLocal<T> {
  private value: T;

  constructor(init: T) {
    this.value = init;
  }

  /** Access the thread-local value via callback (mirrors Rust's .with() API) */
  with<R>(f: (value: T) => R): R {
    return f(this.value);
  }
}
