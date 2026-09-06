// MIRRORS: ankurah/core/src/util/safeset.rs
import { Struct, Result, RwLock, HashSet } from '@ankurah/base';

export class SafeSet<T extends Hash & Eq & Clone & Debug> extends Struct {
  _0: RwLock<HashSet<T>>;

  constructor(_0: RwLock<HashSet<T>>) {
    super();
    this._0 = _0;
  }

  static new<T>(): SafeSet<T> {
    return new SafeSet(new RwLock(new HashSet()));
  }

  insert(value: T): boolean {
    const _t0 = this._0.write();
    try {
      return _t0.value.insert(value);
    } finally {
      _t0.drop();
    }
  }

  remove(value: T): boolean {
    const _t0 = this._0.write();
    try {
      return _t0.value.remove(value);
    } finally {
      _t0.drop();
    }
  }

  contains(value: T): boolean {
    const _t0 = this._0.read();
    try {
      return _t0.value.has(value);
    } finally {
      _t0.drop();
    }
  }

  isEmpty(): boolean {
    const _t0 = this._0.read();
    try {
      return _t0.value.size === 0;
    } finally {
      _t0.drop();
    }
  }

  len(): number {
    const _t0 = this._0.read();
    try {
      return _t0.value.size;
    } finally {
      _t0.drop();
    }
  }

  toVec(): T[] {
    const _t0 = this._0.read();
    try {
      return [...[..._t0.value]];
    } finally {
      _t0.drop();
    }
  }

  toString(): Result {
    return `SafeSet { ${this._0.read()} }`;
  }

  static default<T>(): SafeSet<T> {
    return SafeSet.new();
  }
}

