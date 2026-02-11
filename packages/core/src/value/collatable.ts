// MIRRORS: ankurah/core/src/value/collatable.rs
//
// Collation support for Value (single value).
// Implements the Collatable interface methods as standalone functions operating on Value.
// The Collatable trait/interface itself is defined in the collation module (crate::collation).
// Tuple framing (type tags/lengths) is handled by higher-level encoders.

import type { Value } from './index';

// ── IEEE 754 bit manipulation helpers ────────────────────────────────
// Divergence: Rust has f64::to_bits() returning u64. JS has no direct equivalent,
// so we use DataView to get the raw bits as a BigInt for manipulation. [E8]

const _f64View = new DataView(new ArrayBuffer(8));

function f64ToBits(f: number): bigint {
  _f64View.setFloat64(0, f, false); // big-endian
  return _f64View.getBigUint64(0, false);
}

function numberToBigEndianBytes(n: number): Uint8Array {
  // Encode as i64 big-endian (sign-extended)
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);
  // For negative numbers, we need sign extension via BigInt
  view.setBigInt64(0, BigInt(Math.trunc(n)), false);
  return new Uint8Array(buf);
}

function bigintToBigEndianBytes(n: bigint): Uint8Array {
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);
  view.setBigUint64(0, n, false);
  return new Uint8Array(buf);
}

// ── Collatable methods for Value ─────────────────────────────────────

/** Convert the value to its binary representation for collation.
 *  Mirrors Rust Collatable::to_bytes() impl for Value. */
export function valueCollatableToBytes(value: Value): Uint8Array {
  switch (value.type) {
    case 'String':
      return new TextEncoder().encode(value.value);
    // Use fixed-width big-endian encoding to preserve numeric order across widths
    case 'I16':
      return numberToBigEndianBytes(value.value);
    case 'I32':
      return numberToBigEndianBytes(value.value);
    case 'I64':
      return numberToBigEndianBytes(value.value);
    case 'F64': {
      const f = value.value;
      let bits: bigint;
      if (isNaN(f)) {
        bits = 0xFFFFFFFFFFFFFFFFn; // NaN sorts last
      } else {
        const rawBits = f64ToBits(f);
        if (f >= 0) {
          bits = rawBits ^ (1n << 63n); // Flip sign bit for positive numbers
        } else {
          bits = ~rawBits & 0xFFFFFFFFFFFFFFFFn; // Flip all bits for negative numbers
        }
      }
      return bigintToBigEndianBytes(bits);
    }
    case 'Bool':
      return new Uint8Array([value.value ? 1 : 0]);
    case 'EntityId':
      return new Uint8Array(value.value.toBytes());
    case 'Object':
    case 'Binary':
      return new Uint8Array(value.value);
    case 'Json': {
      try {
        const jsonStr = JSON.stringify(value.value);
        return new TextEncoder().encode(jsonStr);
      } catch {
        return new Uint8Array(0);
      }
    }
  }
}

/** Returns the immediate successor's binary representation if one exists.
 *  Mirrors Rust Collatable::successor_bytes() impl for Value. */
export function valueCollatableSuccessorBytes(value: Value): Uint8Array | null {
  switch (value.type) {
    case 'String': {
      const bytes = new TextEncoder().encode(value.value);
      const result = new Uint8Array(bytes.length + 1);
      result.set(bytes);
      result[bytes.length] = 0;
      return result;
    }
    case 'I16': {
      const I16_MAX = 32767;
      if (value.value === I16_MAX) return null;
      return numberToBigEndianBytes(value.value + 1);
    }
    case 'I32': {
      const I32_MAX = 2147483647;
      if (value.value === I32_MAX) return null;
      return numberToBigEndianBytes(value.value + 1);
    }
    case 'I64': {
      if (value.value === Number.MAX_SAFE_INTEGER) return null;
      return numberToBigEndianBytes(value.value + 1);
    }
    case 'F64': {
      const f = value.value;
      if (isNaN(f) || (f === Infinity)) return null;
      const rawBits = f64ToBits(f);
      let bits: bigint;
      if (f >= 0) {
        bits = rawBits ^ (1n << 63n);
      } else {
        bits = ~rawBits & 0xFFFFFFFFFFFFFFFFn;
      }
      const nextBits = bits + 1n;
      return bigintToBigEndianBytes(nextBits);
    }
    case 'Bool':
      if (value.value) return null;
      return new Uint8Array([1]);
    case 'EntityId': {
      const bytes = new Uint8Array(value.value.toBytes());
      // Increment the byte array (big-endian arithmetic)
      for (let i = 15; i >= 0; i--) {
        if (bytes[i] === 0xFF) {
          bytes[i] = 0;
        } else {
          bytes[i] += 1;
          return bytes;
        }
      }
      return null; // Overflow - already at maximum
    }
    case 'Object':
    case 'Binary':
    case 'Json':
      return null;
  }
}

/** Returns the immediate predecessor's binary representation if one exists.
 *  Mirrors Rust Collatable::predecessor_bytes() impl for Value. */
export function valueCollatablePredecessorBytes(value: Value): Uint8Array | null {
  switch (value.type) {
    case 'String': {
      const bytes = new TextEncoder().encode(value.value);
      if (bytes.length === 0) return null;
      return bytes.slice(0, bytes.length - 1);
    }
    case 'I16': {
      const I16_MIN = -32768;
      if (value.value === I16_MIN) return null;
      return numberToBigEndianBytes(value.value - 1);
    }
    case 'I32': {
      const I32_MIN = -2147483648;
      if (value.value === I32_MIN) return null;
      return numberToBigEndianBytes(value.value - 1);
    }
    case 'I64': {
      if (value.value === Number.MIN_SAFE_INTEGER) return null;
      return numberToBigEndianBytes(value.value - 1);
    }
    case 'F64': {
      const f = value.value;
      if (isNaN(f) || (f === -Infinity)) return null;
      const rawBits = f64ToBits(f);
      let bits: bigint;
      if (f >= 0) {
        bits = rawBits ^ (1n << 63n);
      } else {
        bits = ~rawBits & 0xFFFFFFFFFFFFFFFFn;
      }
      const prevBits = bits - 1n;
      return bigintToBigEndianBytes(prevBits);
    }
    case 'Bool':
      if (value.value) return new Uint8Array([0]);
      return null;
    case 'EntityId': {
      const bytes = new Uint8Array(value.value.toBytes());
      if (bytes.every((b) => b === 0)) return null; // Already at minimum
      // Decrement the byte array (big-endian arithmetic)
      for (let i = 15; i >= 0; i--) {
        if (bytes[i] === 0) {
          bytes[i] = 0xFF;
        } else {
          bytes[i] -= 1;
          return bytes;
        }
      }
      return null; // Should never reach here since we checked for zero above
    }
    case 'Object':
    case 'Binary':
    case 'Json':
      return null;
  }
}

/** Returns true if this value represents a minimum bound in its domain.
 *  Mirrors Rust Collatable::is_minimum() impl for Value. */
export function valueCollatableIsMinimum(value: Value): boolean {
  switch (value.type) {
    case 'String': return value.value.length === 0;
    case 'I16': return value.value === -32768;
    case 'I32': return value.value === -2147483648;
    case 'I64': return value.value === Number.MIN_SAFE_INTEGER;
    case 'F64': return value.value === -Infinity;
    case 'Bool': return !value.value;
    case 'EntityId': return value.value.toBytes().every((b) => b === 0);
    case 'Object':
    case 'Binary':
    case 'Json':
      return false;
  }
}

/** Returns true if this value represents a maximum bound in its domain.
 *  Mirrors Rust Collatable::is_maximum() impl for Value. */
export function valueCollatableIsMaximum(value: Value): boolean {
  switch (value.type) {
    case 'String': return false; // Strings have no theoretical maximum
    case 'I16': return value.value === 32767;
    case 'I32': return value.value === 2147483647;
    case 'I64': return value.value === Number.MAX_SAFE_INTEGER;
    case 'F64': return value.value === Infinity;
    case 'Bool': return value.value;
    case 'EntityId': return value.value.toBytes().every((b) => b === 0xFF);
    case 'Object':
    case 'Binary':
    case 'Json':
      return false;
  }
}

/** Compare two values in the collation order. Returns -1, 0, or 1.
 *  Mirrors Rust Collatable::compare() default impl. */
export function valueCollatableCompare(a: Value, b: Value): number {
  const aBytes = valueCollatableToBytes(a);
  const bBytes = valueCollatableToBytes(b);
  const len = Math.min(aBytes.length, bBytes.length);
  for (let i = 0; i < len; i++) {
    if (aBytes[i] < bBytes[i]) return -1;
    if (aBytes[i] > bBytes[i]) return 1;
  }
  if (aBytes.length < bBytes.length) return -1;
  if (aBytes.length > bBytes.length) return 1;
  return 0;
}
