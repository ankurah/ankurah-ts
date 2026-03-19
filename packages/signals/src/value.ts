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
  private _0: ValueInner<T>;

  constructor(value: T) {
    super();
    this._0 = { value };
  }

  clone(): ValueCell<T> {
    const cloned = new ValueCell<T>(undefined as any);
    cloned._0 = this._0;
    return cloned;
  }

  set(value: T): void {
    this._0.value = value;
  }

  with<R>(f: (value: T) => R): R {
    return f(this._0.value);
  }

  setWith<R>(value: T, f: (value: T) => R): R {
    this._0.value = value;
    return f(this._0.value);
  }

  /** Create a read-only view of this value */
  readvalue(): ReadValueCell<T> {
    return new ReadValueCell<T>(this._0);
  }

  // Alias: existing callers use readValue (camelCase)
  readValue(): ReadValueCell<T> {
    return this.readvalue();
  }

  // impl<T: Clone> ValueCell<T>
  value(): T {
    return this._0.value;
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
  private _0: ValueInner<T>;

  /** @internal */
  constructor(_0: ValueInner<T>) {
    super();
    this._0 = _0;
  }

  clone(): ReadValueCell<T> {
    return new ReadValueCell<T>(this._0);
  }

  with<R>(f: (value: T) => R): R {
    return f(this._0.value);
  }

  // impl<T: Clone> ReadValueCell<T>
  value(): T {
    return this._0.value;
  }

  // Alias: existing callers use getValue
  getValue(): T {
    return this.value();
  }
}
