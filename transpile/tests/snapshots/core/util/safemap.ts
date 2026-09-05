// MIRRORS: ankurah/core/src/util/safemap.rs
import { Struct, Result, RwLock, invokeRef, dropOwned, HashMap, HashSet } from '@ankurah/base';

export class SafeMap<K extends Hash & Eq & Clone & Debug, V extends Clone & Default & Debug> extends Struct {
  _0: RwLock<HashMap<K, V>>;

  constructor(_0: RwLock<HashMap<K, V>>) {
    super();
    this._0 = _0;
  }

  static new<K, V>(): SafeMap<K, V> {
    return new SafeMap(new RwLock(new HashMap()));
  }

  insert(key: K, value: V): void {
    const _t0 = this._0.write();
    try {
      _t0.value.set(key, value);
    } finally {
      _t0.drop();
    }
  }

  remove(key: K): V | null {
    const _t0 = this._0.write();
    try {
      return _t0.value.remove(key);
    } finally {
      _t0.drop();
    }
  }

  retain(cb: (arg0: K, arg1: V) => boolean): void {
    const _t0 = this._0.write();
    try {
      { for (const [_k, _v] of _t0.value) { if (!(((k, v) => invokeRef(cb, k, v))(_k, _v))) _t0.value.delete(_k); } };
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

  clear(): void {
    const _t0 = this._0.write();
    try {
      _t0.value.clear();
    } finally {
      _t0.drop();
    }
  }

  containsKey(key: K): boolean {
    const _t0 = this._0.read();
    try {
      return _t0.value.has(key);
    } finally {
      _t0.drop();
    }
  }

  get(k: K): V | null {
    const _t0 = this._0.read();
    try {
      return _t0.value.get(k);
    } finally {
      _t0.drop();
    }
  }

  getList(k: K[]): [K, V | null][] {
    const read = this._0.read();
    try {
      return [...k].map((k) => {
        const v = read.value.get(k);
        return [k, v];
      });
    } finally {
      read.drop();
    }
  }

  getOrDefault(k: K): V {
    const _t0 = this._0.write();
    try {
      return _t0.value.entry(k).match({
        Occupied: (v) => {
          const o = v._0;
          return o.get().clone();
        },
        Vacant: (_v) => {
          const v = _v._0;
          return v.insert(Default.default()).clone();
        },
      });
    } finally {
      _t0.drop();
    }
  }

  toVec(): [K, V][] {
    const _t0 = this._0.read();
    try {
      return [..._t0.value].map(([k, v]) => [k.clone(), v.clone()]);
    } finally {
      _t0.drop();
    }
  }

  keys(): K[] {
    const _t0 = this._0.read();
    try {
      return [..._t0.value.keys()];
    } finally {
      _t0.drop();
    }
  }

  values(): V[] {
    const _t0 = this._0.read();
    try {
      return [..._t0.value.values()];
    } finally {
      _t0.drop();
    }
  }

  push(key: K, value: H): void {
    const _t0 = this._0.write();
    try {
      _t0.value.entry(key).orDefault(() => []).push(value);
    } finally {
      _t0.drop();
    }
  }

  removeEq(key: K, value: H): void {
    const _t0 = this._0.write();
    try {
      {
        const _v = _t0.value.get(key);
        if (_v != null) {
          const v = _v;
          (($xs) => { let $at = 0; for (let $i = 0; $i < $xs.length; $i++) { if (((h) => h !== value)($xs[$i])) { $xs[$at++] = $xs[$i]; } else { dropOwned($xs[$i]); } } $xs.length = $at; })(v);
        }
      }
    } finally {
      _t0.drop();
    }
  }

  setInsert(key: K, value: H): void {
    const _t0 = this._0.write();
    try {
      _t0.value.entry(key).orDefault(() => new HashSet()).add(value);
    } finally {
      _t0.drop();
    }
  }

  setRemove(key: K, value: H): boolean {
    const _t0 = this._0.write();
    try {
      const _v = _t0.value.get(key);
      if (_v != null) {
        const v = _v;
        return v.remove(value);
      } else {
        return false;
      }
    } finally {
      _t0.drop();
    }
  }

  toString(): Result {
    return `SafeMap { ${this._0.read()} }`;
  }

  static default<K, V>(): SafeMap<K, V> {
    return new SafeMap(new RwLock(new HashMap()));
  }
}

