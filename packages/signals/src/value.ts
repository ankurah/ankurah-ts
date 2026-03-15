// MIRRORS: ankurah/signals/src/value.rs
import { Struct } from '@ankurah/base';

// Divergence: Rust uses Arc<RwLock<T>> for shared mutable state; TS uses a plain value holder
// since JS is single-threaded [E8]. Clone semantics are preserved by sharing an inner container.

// TODO create an inner that Both can share. then use deref to try to golf this a bit

interface ValueInner<T> {
  value: T;
}

/**
 * A mutable value container.
 * In Rust this wraps Arc<RwLock<T>>; cloning shares the underlying storage.
 * In TS we share a mutable inner object so clones see the same state [E8].
 */
export class ValueCell<T> extends Struct {
  private inner: ValueInner<T>;

  constructor(value: T) {
    super();
    this.inner = { value };
  }

  clone(): ValueCell<T> {
    const cloned = new ValueCell<T>(undefined as any);
    cloned.inner = this.inner;
    return cloned;
  }

  set(value: T): void {
    this.inner.value = value;
  }

  with<R>(f: (value: T) => R): R {
    return f(this.inner.value);
  }

  setWith<R>(value: T, f: (value: T) => R): R {
    this.inner.value = value;
    return f(this.inner.value);
  }

  /** Create a read-only view of this value */
  readvalue(): ReadValueCell<T> {
    return new ReadValueCell<T>(this.inner);
  }

  // Alias: existing callers use readValue (camelCase)
  readValue(): ReadValueCell<T> {
    return this.readvalue();
  }

  // impl<T: Clone> ValueCell<T>
  value(): T {
    return this.inner.value;
  }

  // Alias: existing callers use getValue
  getValue(): T {
    return this.value();
  }
}

/**
 * A read-only value container that shares storage with ValueCell<T>.
 * In Rust this shares an Arc<RwLock<T>> with ValueCell.
 * In TS, we share the same inner object [E8].
 */
export class ReadValueCell<T> extends Struct {
  /** @internal */
  private inner: ValueInner<T>;

  /** @internal */
  constructor(inner: ValueInner<T>) {
    super();
    this.inner = inner;
  }

  clone(): ReadValueCell<T> {
    return new ReadValueCell<T>(this.inner);
  }

  with<R>(f: (value: T) => R): R {
    return f(this.inner.value);
  }

  // impl<T: Clone> ReadValueCell<T>
  value(): T {
    return this.inner.value;
  }

  // Alias: existing callers use getValue
  getValue(): T {
    return this.value();
  }
}
