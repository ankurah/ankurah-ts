// MIRRORS: ankurah/signals/src/value.rs

/**
 * A mutable value container.
 * In Rust this uses Arc<RwLock<T>> for thread safety.
 * In JS, single-threaded means this is just a plain value holder [E8].
 */
export class ValueCell<T> {
  private value: T;

  constructor(value: T) {
    this.value = value;
  }

  /** Set the current value */
  set(value: T): void {
    this.value = value;
  }

  /** Call a function with a reference to the current value */
  with<R>(f: (value: T) => R): R {
    return f(this.value);
  }

  /** Set the value and call a function with the new value */
  setWith<R>(value: T, f: (value: T) => R): R {
    this.value = value;
    return f(this.value);
  }

  /** Get the current value (in Rust this requires Clone; in JS just return the reference) */
  getValue(): T {
    return this.value;
  }

  /** Create a read-only view of this value (shares storage) */
  readValue(): ReadValueCell<T> {
    return new ReadValueCell(this);
  }
}

/**
 * A read-only value container that shares storage with ValueCell<T>.
 * In Rust this shares an Arc<RwLock<T>> with ValueCell.
 * In JS, we hold a reference to the parent ValueCell [E8].
 */
export class ReadValueCell<T> {
  /** @internal - holds reference to the source ValueCell for shared storage */
  private source: ValueCell<T>;

  /** @internal */
  constructor(source: ValueCell<T>) {
    this.source = source;
  }

  /** Call a function with a reference to the current value */
  with<R>(f: (value: T) => R): R {
    return this.source.with(f);
  }

  /** Get the current value */
  getValue(): T {
    return this.source.getValue();
  }
}
