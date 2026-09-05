// MIRRORS: ankurah/signals/src/value.rs
import { Struct, Arc, RwLock, invoke, invokeRef, dropOwned } from '@ankurah/base';

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
    let current = this._0.value.write();
    try {
      current.value = value;
    } finally {
      current.drop();
    }
  }

  with<R>(f: (arg0: T) => R): R {
    const guard = this._0.value.read();
    try {
      return invoke(f, guard.value);
    } finally {
      guard.drop();
    }
  }

  setWith<R>(value: T, f: (arg0: T) => R): R {
    try {
      let current = this._0.value.write();
      try {
        current.value = value;
        return invokeRef(f, current.value);
      } finally {
        current.drop();
      }
    } finally {
      dropOwned(f);
    }
  }

  readvalue(): ReadValueCell<T> {
    return new ReadValueCell(this._0.clone());
  }

  value(): T {
    const _t0 = this._0.value.read();
    try {
      return _t0.value.clone();
    } finally {
      _t0.drop();
    }
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
    const guard = this._0.value.read();
    try {
      return invoke(f, guard.value);
    } finally {
      guard.drop();
    }
  }

  value(): T {
    const _t0 = this._0.value.read();
    try {
      return _t0.value.clone();
    } finally {
      _t0.drop();
    }
  }

  clone(): ReadValueCell<T> {
    return new ReadValueCell(this._0.clone());
  }
}

