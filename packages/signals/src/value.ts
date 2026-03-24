// MIRRORS: ankurah/signals/src/value.rs
import { Struct, Arc, RwLock } from '@ankurah/base';

export class ValueCell<T extends Clone> extends Struct {
  _0: Arc<RwLock<T>>;

  constructor(_0: Arc<RwLock<T>>) {
    super();
    this._0 = _0;
  }

  static new<T>(value: T): ValueCell<T> {
    return new ValueCell(Arc.new(new RwLock(value)));
  }

  set(value: T): void {
    let current = this._0.value.write().value;
    current.value = value;
    current.drop();
  }

  with<R>(f: (arg0: T) => R): R {
    const guard = this._0.value.read().value;
    const _ret = f(guard);
    guard.drop();
    return _ret;
  }

  setWith<R>(value: T, f: (arg0: T) => R): R {
    let current = this._0.value.write().value;
    current.value = value;
    const _ret = f(current);
    current.drop();
    return _ret;
  }

  readvalue(): ReadValueCell<T> {
    return new ReadValueCell(this._0.clone());
  }

  value(): T {
    return this._0.value.read().value.clone();
  }

  clone(): ValueCell<T> {
    return new ValueCell(this._0.clone());
  }
}

export class ReadValueCell<T extends Clone> extends Struct {
  _0: Arc<RwLock<T>>;

  constructor(_0: Arc<RwLock<T>>) {
    super();
    this._0 = _0;
  }

  with<R>(f: (arg0: T) => R): R {
    const guard = this._0.value.read().value;
    const _ret = f(guard);
    guard.drop();
    return _ret;
  }

  value(): T {
    return this._0.value.read().value.clone();
  }

  clone(): ReadValueCell<T> {
    return new ReadValueCell(this._0.clone());
  }
}

