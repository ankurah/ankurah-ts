// MIRRORS: ankurah/core/src/value/collatable.rs
import { checkedAdd, checkedSub } from '@ankurah/base';
import { Collatable } from '../collation';
import { Json } from '../property/value/json';
import { EntityId } from '@ankurah/proto';

export function Value_toBytes(self: Value): Uint8Array {
  return self.match({
    String: (v) => {
      const s = v._0;
      return s.asBytes().slice();
    },
    I16: (v) => {
      const x = v._0;
      return (BigInt(x)).toBeBytes().slice();
    },
    I32: (v) => {
      const x = v._0;
      return (BigInt(x)).toBeBytes().slice();
    },
    I64: (v) => {
      const x = v._0;
      return x.toBeBytes().slice();
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
      const entityId = v._0;
      return entityId.toBytes().slice();
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

export function Value_successorBytes(self: Value): Uint8Array | null {
  return self.match({
    String: (v) => {
      const s = v._0;
      let bytes = s.asBytes().slice();
      bytes.push(0);
      return bytes;
    },
    I16: (v) => {
      const x = v._0;
      if (x === i16.MAX) {
        return null;
      } else {
        return (checkedAdd((BigInt(x)), 1n, 'i64')).toBeBytes().slice();
      }
    },
    I32: (v) => {
      const x = v._0;
      if (x === i32.MAX) {
        return null;
      } else {
        return (checkedAdd((BigInt(x)), 1n, 'i64')).toBeBytes().slice();
      }
    },
    I64: (v) => {
      const x = v._0;
      if (x === i64.MAX) {
        return null;
      } else {
        return (checkedAdd(x, 1n, 'i64')).toBeBytes().slice();
      }
    },
    F64: (v) => {
      const f = v._0;
      if (Number.isNaN(f) || ((!Number.isFinite(f) && !Number.isNaN(f)) && f > 0.0)) {
        return null;
      } else {
        const bits = f >= 0.0 ? f.toBits() ^ (BigInt.asUintN(64, (1n << 63n))) : BigInt.asUintN(64, ~f.toBits());
        const nextBits = checkedAdd(bits, 1n, 'u64');
        return nextBits.toBeBytes().slice();
      }
    },
    Bool: (v) => {
      const b = v._0;
      if (b) {
        return null;
      } else {
        return [1];
      }
    },
    EntityId: (v) => {
      const entityId = v._0;
      let bytes = entityId.toBytes();
      for (const i of (undefined /* range 0..16 */).rev()) {
        if (bytes[i] === 255) {
          bytes[i] = 0;
        } else {
          bytes[i] = checkedAdd(bytes[i], 1, 'u8');
          return bytes.slice();
        }
      }
      return null;
    },
    Object: (v) => null,
    Binary: (v) => null,
    Json: (v) => null,
  });
}

export function Value_predecessorBytes(self: Value): Uint8Array | null {
  return self.match({
    String: (v) => {
      const s = v._0;
      const bytes = s.asBytes();
      if (bytes.length === 0) {
        return null;
      } else {
        return bytes.slice(0, checkedSub(bytes.length, 1, 'usize')).slice();
      }
    },
    I16: (v) => {
      const x = v._0;
      if (x === i16.MIN) {
        return null;
      } else {
        return (checkedSub((BigInt(x)), 1n, 'i64')).toBeBytes().slice();
      }
    },
    I32: (v) => {
      const x = v._0;
      if (x === i32.MIN) {
        return null;
      } else {
        return (checkedSub((BigInt(x)), 1n, 'i64')).toBeBytes().slice();
      }
    },
    I64: (v) => {
      const x = v._0;
      if (x === i64.MIN) {
        return null;
      } else {
        return (checkedSub(x, 1n, 'i64')).toBeBytes().slice();
      }
    },
    F64: (v) => {
      const f = v._0;
      if (Number.isNaN(f) || ((!Number.isFinite(f) && !Number.isNaN(f)) && f < 0.0)) {
        return null;
      } else {
        const bits = f >= 0.0 ? f.toBits() ^ (BigInt.asUintN(64, (1n << 63n))) : BigInt.asUintN(64, ~f.toBits());
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
      const entityId = v._0;
      let bytes = entityId.toBytes();
      if (bytes === Array(16).fill(0)) {
        return null;
      } else {
        for (const i of (undefined /* range 0..16 */).rev()) {
          if (bytes[i] === 0) {
            bytes[i] = 255;
          } else {
            bytes[i] = checkedSub(bytes[i], 1, 'u8');
            return bytes.slice();
          }
        }
        return null;
      }
    },
    Object: (v) => null,
    Binary: (v) => null,
    Json: (v) => null,
  });
}

export function Value_isMinimum(self: Value): boolean {
  return self.match({
    String: (v) => {
      const s = v._0;
      return s.length === 0;
    },
    I16: (v) => {
      const x = v._0;
      return x === i16.MIN;
    },
    I32: (v) => {
      const x = v._0;
      return x === i32.MIN;
    },
    I64: (v) => {
      const x = v._0;
      return x === i64.MIN;
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
      const entityId = v._0;
      return entityId.toBytes() === Array(16).fill(0);
    },
    Object: (v) => false,
    Binary: (v) => false,
    Json: (v) => false,
  });
}

export function Value_isMaximum(self: Value): boolean {
  return self.match({
    String: (v) => false,
    I16: (v) => {
      const x = v._0;
      return x === i16.MAX;
    },
    I32: (v) => {
      const x = v._0;
      return x === i32.MAX;
    },
    I64: (v) => {
      const x = v._0;
      return x === i64.MAX;
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
      const entityId = v._0;
      return entityId.toBytes() === Array(16).fill(255);
    },
    Object: (v) => false,
    Binary: (v) => false,
    Json: (v) => false,
  });
}

