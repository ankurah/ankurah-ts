// MIRRORS: ankurah/core/src/collation.rs
import { Enum, checkedAdd, checkedSub } from '@ankurah/base';
import { EntityId } from '@ankurah/proto';
import { Json } from './property/value/json';

export type RangeBoundV = {
  Included: { _0: T };
  Excluded: { _0: T };
  Unbounded: {};
};

export class RangeBound<T> extends Enum<RangeBoundV> {

  clone(): RangeBound<T> {
    return this.match({
      Included: (v) => new RangeBound('Included', { _0: v._0.clone() }),
      Excluded: (v) => new RangeBound('Excluded', { _0: v._0.clone() }),
      Unbounded: () => new RangeBound('Unbounded', {}),
    });
  }

  equals(other: RangeBound<T>): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'Included': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'Excluded': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      Included: (v) => `Included(${v._0})`,
      Excluded: (v) => `Excluded(${v._0})`,
      Unbounded: () => 'Unbounded',
    });
  }
}

export abstract class Collatable {
  abstract toBytes(): Uint8Array;
  abstract successorBytes(): Uint8Array | null;
  abstract predecessorBytes(): Uint8Array | null;
  abstract isMinimum(): boolean;
  abstract isMaximum(): boolean;
  compare(other: Self): number {
    return Collatable_dispatch_toBytes(this).compareTo(Collatable_dispatch_toBytes(other));
  }
  isInRange(lower: RangeBound<Self>, upper: RangeBound<Self>): boolean {
    const _v = [lower, upper];
    if ((_v[0].is('Included')) && (_v[1].is('Included'))) {
      const { _0: l } = _v[0].value;
      const { _0: u } = _v[1].value;
      return Collatable_dispatch_compare(this, l) !== -1 && Collatable_dispatch_compare(this, u) !== 1;
    } else if ((_v[0].is('Included')) && (_v[1].is('Excluded'))) {
      const { _0: l } = _v[0].value;
      const { _0: u } = _v[1].value;
      return Collatable_dispatch_compare(this, l) !== -1 && Collatable_dispatch_compare(this, u) === -1;
    } else if ((_v[0].is('Excluded')) && (_v[1].is('Included'))) {
      const { _0: l } = _v[0].value;
      const { _0: u } = _v[1].value;
      return Collatable_dispatch_compare(this, l) === 1 && Collatable_dispatch_compare(this, u) !== 1;
    } else if ((_v[0].is('Excluded')) && (_v[1].is('Excluded'))) {
      const { _0: l } = _v[0].value;
      const { _0: u } = _v[1].value;
      return Collatable_dispatch_compare(this, l) === 1 && Collatable_dispatch_compare(this, u) === -1;
    } else if ((_v[0].is('Unbounded')) && (_v[1].is('Included'))) {
      const { _0: u } = _v[1].value;
      return Collatable_dispatch_compare(this, u) !== 1;
    } else if ((_v[0].is('Unbounded')) && (_v[1].is('Excluded'))) {
      const { _0: u } = _v[1].value;
      return Collatable_dispatch_compare(this, u) === -1;
    } else if ((_v[0].is('Included')) && (_v[1].is('Unbounded'))) {
      const { _0: l } = _v[0].value;
      return Collatable_dispatch_compare(this, l) !== -1;
    } else if ((_v[0].is('Excluded')) && (_v[1].is('Unbounded'))) {
      const { _0: l } = _v[0].value;
      return Collatable_dispatch_compare(this, l) === 1;
    } else {
      return true;
    }
  }
}

export function Literal_toBytes(self: Literal): Uint8Array {
  return self.match({
    String: (v) => {
      const s = v._0;
      return s.asBytes().slice();
    },
    I16: (v) => {
      const i = v._0;
      return i.toBeBytes().slice();
    },
    I32: (v) => {
      const i = v._0;
      return i.toBeBytes().slice();
    },
    I64: (v) => {
      const i = v._0;
      return i.toBeBytes().slice();
    },
    F64: (v) => {
      const f = v._0;
      const bits = (() => {
        if (Number.isNaN(f)) {
          return u64.MAX;
        } else {
          const bits = f.toBits();
          if (f >= 0.0) {
            return bits ^ (BigInt.asUintN(64, (1n << 63n)));
          } else {
            return BigInt.asUintN(64, ~bits);
          }
        }
      })();
      return bits.toBeBytes().toVec();
    },
    Bool: (v) => {
      const b = v._0;
      return [Number(b)] as any;
    },
    EntityId: (v) => {
      const ulid = v._0;
      return ulid.toBytes().slice();
    },
    Object: (v) => {
      const bytes = v._0;
      return bytes.clone();
    },
    Binary: (v) => {
      const bytes = v._0;
      return bytes.clone();
    },
    Json: (v) => {
      const json = v._0;
      return serdeJson.toVec(json).unwrapOrDefault();
    },
  });
}

export function Literal_successorBytes(self: Literal): Uint8Array | null {
  return self.match({
    String: (v) => {
      const s = v._0;
      if (s.length === 0) {
        let bytes = s.asBytes().slice();
        bytes.push(0);
        return bytes;
      } else {
        let bytes = s.asBytes().slice();
        bytes.push(0);
        return bytes;
      }
    },
    I16: (v) => {
      const i = v._0;
      if (i === i16.MAX) {
        return null;
      } else {
        return (checkedAdd(i, 1, 'i16')).toBeBytes().slice();
      }
    },
    I32: (v) => {
      const i = v._0;
      if (i === i32.MAX) {
        return null;
      } else {
        return (checkedAdd(i, 1, 'i32')).toBeBytes().slice();
      }
    },
    I64: (v) => {
      const i = v._0;
      if (i === i64.MAX) {
        return null;
      } else {
        return (checkedAdd(i, 1n, 'i64')).toBeBytes().slice();
      }
    },
    F64: (v) => {
      const f = v._0;
      if (Number.isNaN(f) || ((!Number.isFinite(f) && !Number.isNaN(f)) && f > 0.0)) {
        return null;
      } else {
        const bits = (f >= 0.0 ? f.toBits() ^ (BigInt.asUintN(64, (1n << 63n))) : BigInt.asUintN(64, ~f.toBits()));
        const nextBits = checkedAdd(bits, 1n, 'u64');
        return nextBits.toBeBytes().slice();
      }
    },
    Bool: (v) => {
      const b = v._0;
      if (!b) {
        return null;
      } else {
        return [1];
      }
    },
    EntityId: (v) => {
      const ulid = v._0;
      let bytes = ulid.toBytes();
      for (const i of (undefined /* range 0..bytes.length */).rev()) {
        if (bytes[i] < 255) {
          bytes[i] = checkedAdd(bytes[i], 1, 'u8');
          for (const j of undefined /* range (checkedAdd(i, 1, 'usize'))..bytes.length */) {
            bytes[j] = 0;
          }
          return bytes.slice();
        }
      }
      return null;
    },
    Object: (v) => {
      const bytes = v._0;
      let bytes_1 = bytes.clone();
      for (const i of (undefined /* range 0..bytes_1.length */).rev()) {
        if (bytes_1[i] < 255) {
          bytes_1[i] = checkedAdd(bytes_1[i], 1, 'u8');
          for (const j of undefined /* range (checkedAdd(i, 1, 'usize'))..bytes_1.length */) {
            bytes_1[j] = 0;
          }
          return bytes_1;
        }
      }
      bytes_1.push(0);
      return bytes_1;
    },
    Binary: (v) => {
      const bytes = v._0;
      let bytes_1 = bytes.clone();
      for (const i of (undefined /* range 0..bytes_1.length */).rev()) {
        if (bytes_1[i] < 255) {
          bytes_1[i] = checkedAdd(bytes_1[i], 1, 'u8');
          for (const j of undefined /* range (checkedAdd(i, 1, 'usize'))..bytes_1.length */) {
            bytes_1[j] = 0;
          }
          return bytes_1;
        }
      }
      bytes_1.push(0);
      return bytes_1;
    },
    Json: (v) => null,
  });
}

export function Literal_predecessorBytes(self: Literal): Uint8Array | null {
  return self.match({
    String: (v) => {
      const s = v._0;
      if (s.length === 0) {
        return null;
      } else {
        const bytes = s.asBytes();
        return bytes.slice(0, checkedSub(bytes.length, 1, 'usize')).slice();
      }
    },
    I16: (v) => {
      const i = v._0;
      if (i === i16.MIN) {
        return null;
      } else {
        return (checkedSub(i, 1, 'i16')).toBeBytes().slice();
      }
    },
    I32: (v) => {
      const i = v._0;
      if (i === i32.MIN) {
        return null;
      } else {
        return (checkedSub(i, 1, 'i32')).toBeBytes().slice();
      }
    },
    I64: (v) => {
      const i = v._0;
      if (i === i64.MIN) {
        return null;
      } else {
        return (checkedSub(i, 1n, 'i64')).toBeBytes().slice();
      }
    },
    F64: (v) => {
      const f = v._0;
      if (Number.isNaN(f) || ((!Number.isFinite(f) && !Number.isNaN(f)) && f < 0.0)) {
        return null;
      } else {
        const bits = (f >= 0.0 ? f.toBits() ^ (BigInt.asUintN(64, (1n << 63n))) : BigInt.asUintN(64, ~f.toBits()));
        const prevBits = checkedSub(bits, 1n, 'u64');
        return prevBits.toBeBytes().slice();
      }
    },
    Bool: (v) => {
      const b = v._0;
      if (b) {
        return [0];
      } else {
        return null;
      }
    },
    EntityId: (v) => {
      const ulid = v._0;
      let bytes = ulid.toBytes();
      for (const i of (undefined /* range 0..bytes.length */).rev()) {
        if (bytes[i] > 0) {
          bytes[i] = checkedSub(bytes[i], 1, 'u8');
          for (const j of undefined /* range (checkedAdd(i, 1, 'usize'))..bytes.length */) {
            bytes[j] = 255;
          }
          return bytes.slice();
        }
      }
      return null;
    },
    Object: (v) => {
      const bytes = v._0;
      if (bytes.length === 0) {
        return null;
      } else {
        let bytes_1 = bytes.clone();
        for (const i of (undefined /* range 0..bytes_1.length */).rev()) {
          if (bytes_1[i] > 0) {
            bytes_1[i] = checkedSub(bytes_1[i], 1, 'u8');
            for (const j of undefined /* range (checkedAdd(i, 1, 'usize'))..bytes_1.length */) {
              bytes_1[j] = 255;
            }
            return bytes_1;
          }
        }
        if (bytes_1.length > 1) {
          bytes_1.pop();
          return bytes_1;
        } else {
          return null;
        }
      }
    },
    Binary: (v) => {
      const bytes = v._0;
      if (bytes.length === 0) {
        return null;
      } else {
        let bytes_1 = bytes.clone();
        for (const i of (undefined /* range 0..bytes_1.length */).rev()) {
          if (bytes_1[i] > 0) {
            bytes_1[i] = checkedSub(bytes_1[i], 1, 'u8');
            for (const j of undefined /* range (checkedAdd(i, 1, 'usize'))..bytes_1.length */) {
              bytes_1[j] = 255;
            }
            return bytes_1;
          }
        }
        if (bytes_1.length > 1) {
          bytes_1.pop();
          return bytes_1;
        } else {
          return null;
        }
      }
    },
    Json: (v) => null,
  });
}

export function Literal_isMinimum(self: Literal): boolean {
  return self.match({
    String: (v) => {
      const s = v._0;
      return s.length === 0;
    },
    I16: (v) => {
      const i = v._0;
      return i === i16.MIN;
    },
    I32: (v) => {
      const i = v._0;
      return i === i32.MIN;
    },
    I64: (v) => {
      const i = v._0;
      return i === i64.MIN;
    },
    F64: (v) => {
      const f = v._0;
      return f === f64.NEG_INFINITY;
    },
    Bool: (v) => {
      const b = v._0;
      return !b;
    },
    EntityId: (v) => {
      const ulid = v._0;
      return [...ulid.toBytes()].every((b) => b === 0) as any;
    },
    Object: (v) => {
      const bytes = v._0;
      return bytes.length === 0;
    },
    Binary: (v) => {
      const bytes = v._0;
      return bytes.length === 0;
    },
    Json: (v) => false,
  });
}

export function Literal_isMaximum(self: Literal): boolean {
  return self.match({
    String: (v) => false,
    I16: (v) => {
      const i = v._0;
      return i === i16.MAX;
    },
    I32: (v) => {
      const i = v._0;
      return i === i32.MAX;
    },
    I64: (v) => {
      const i = v._0;
      return i === i64.MAX;
    },
    F64: (v) => {
      const f = v._0;
      return f === f64.INFINITY;
    },
    Bool: (v) => {
      const b = v._0;
      return b;
    },
    EntityId: (v) => {
      const ulid = v._0;
      return [...ulid.toBytes()].every((b) => b === 255) as any;
    },
    Object: (v) => false,
    Binary: (v) => false,
    Json: (v) => false,
  });
}

export function Str_toBytes(self: string): Uint8Array {
  return self.asBytes().slice();
}

export function Str_successorBytes(self: string): Uint8Array | null {
  if (Str_isMaximum(self)) {
    return null;
  } else {
    let bytes = self.asBytes().slice();
    bytes.push(0);
    return bytes;
  }
}

export function Str_predecessorBytes(self: string): Uint8Array | null {
  if (Str_isMinimum(self)) {
    return null;
  } else {
    const bytes = self.asBytes();
    if (bytes.length === 0) {
      return null;
    } else {
      return bytes.slice(0, checkedSub(bytes.length, 1, 'usize')).slice();
    }
  }
}

export function Str_isMinimum(self: string): boolean {
  return self.length === 0;
}

export function Str_isMaximum(self: string): boolean {
  return false;
}

export function I64_toBytes(self: bigint): Uint8Array {
  return self.toBeBytes().slice();
}

export function I64_successorBytes(self: bigint): Uint8Array | null {
  if (self === i64.MAX) {
    return null;
  } else {
    return (checkedAdd(self, 1n, 'i64')).toBeBytes().slice();
  }
}

export function I64_predecessorBytes(self: bigint): Uint8Array | null {
  if (self === i64.MIN) {
    return null;
  } else {
    return (checkedSub(self, 1n, 'i64')).toBeBytes().slice();
  }
}

export function I64_isMinimum(self: bigint): boolean {
  return self === i64.MIN;
}

export function I64_isMaximum(self: bigint): boolean {
  return self === i64.MAX;
}

export function F64_toBytes(self: number): Uint8Array {
  const bits = (() => {
    if (Number.isNaN(self)) {
      return u64.MAX;
    } else {
      const bits = self.toBits();
      if (self >= 0.0) {
        return bits ^ (BigInt.asUintN(64, (1n << 63n)));
      } else {
        return BigInt.asUintN(64, ~bits);
      }
    }
  })();
  return bits.toBeBytes().toVec();
}

export function F64_successorBytes(self: number): Uint8Array | null {
  if (Number.isNaN(self) || ((!Number.isFinite(self) && !Number.isNaN(self)) && self > 0.0)) {
    return null;
  } else {
    const bits = (self >= 0.0 ? self.toBits() ^ (BigInt.asUintN(64, (1n << 63n))) : BigInt.asUintN(64, ~self.toBits()));
    const nextBits = checkedAdd(bits, 1n, 'u64');
    return nextBits.toBeBytes().slice();
  }
}

export function F64_predecessorBytes(self: number): Uint8Array | null {
  if (Number.isNaN(self) || ((!Number.isFinite(self) && !Number.isNaN(self)) && self < 0.0)) {
    return null;
  } else {
    const bits = (self >= 0.0 ? self.toBits() ^ (BigInt.asUintN(64, (1n << 63n))) : BigInt.asUintN(64, ~self.toBits()));
    const prevBits = checkedSub(bits, 1n, 'u64');
    return prevBits.toBeBytes().slice();
  }
}

export function F64_isMinimum(self: number): boolean {
  return self === f64.NEG_INFINITY;
}

export function F64_isMaximum(self: number): boolean {
  return self === f64.INFINITY;
}

export function EntityId_toBytes(self: EntityId): Uint8Array {
  return self.toBytes().slice();
}

export function EntityId_successorBytes(self: EntityId): Uint8Array | null {
  if (EntityId_isMaximum(self)) {
    return null;
  } else {
    let bytes = self.toBytes();
    for (const i of (undefined /* range 0..bytes.length */).rev()) {
      if (bytes[i] < 255) {
        bytes[i] = checkedAdd(bytes[i], 1, 'u8');
        for (const j of undefined /* range (checkedAdd(i, 1, 'usize'))..bytes.length */) {
          bytes[j] = 0;
        }
        return bytes.slice();
      }
    }
    return null;
  }
}

export function EntityId_predecessorBytes(self: EntityId): Uint8Array | null {
  if (EntityId_isMinimum(self)) {
    return null;
  } else {
    let bytes = self.toBytes();
    for (const i of (undefined /* range 0..bytes.length */).rev()) {
      if (bytes[i] > 0) {
        bytes[i] = checkedSub(bytes[i], 1, 'u8');
        for (const j of undefined /* range (checkedAdd(i, 1, 'usize'))..bytes.length */) {
          bytes[j] = 255;
        }
        return bytes.slice();
      }
    }
    return null;
  }
}

export function EntityId_isMinimum(self: EntityId): boolean {
  return [...self.toBytes()].every((b) => b === 0);
}

export function EntityId_isMaximum(self: EntityId): boolean {
  return [...self.toBytes()].every((b) => b === 255);
}

export function Collatable_dispatch_compare(self: unknown, other: Self): number {
  if (self instanceof Literal) return Literal_compare(self as any, other);
  if (self instanceof EntityId) return EntityId_compare(self as any, other);
  if (self instanceof Value) return Value_compare(self as any, other);
  throw new Error(`BUG: no Collatable impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

export function Collatable_dispatch_predecessorBytes(self: unknown): Uint8Array | null {
  if (self instanceof Literal) return Literal_predecessorBytes(self as any);
  if (self instanceof EntityId) return EntityId_predecessorBytes(self as any);
  if (self instanceof Value) return Value_predecessorBytes(self as any);
  throw new Error(`BUG: no Collatable impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

export function Collatable_dispatch_successorBytes(self: unknown): Uint8Array | null {
  if (self instanceof Literal) return Literal_successorBytes(self as any);
  if (self instanceof EntityId) return EntityId_successorBytes(self as any);
  if (self instanceof Value) return Value_successorBytes(self as any);
  throw new Error(`BUG: no Collatable impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

export function Collatable_dispatch_toBytes(self: unknown): Uint8Array {
  if (self instanceof Literal) return Literal_toBytes(self as any);
  if (self instanceof EntityId) return EntityId_toBytes(self as any);
  if (self instanceof Value) return Value_toBytes(self as any);
  throw new Error(`BUG: no Collatable impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

