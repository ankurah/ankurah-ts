// MIRRORS: ankurah/core/src/collation.rs

import { Literal } from '@ankurah/ankql';
import type { EntityId } from '@ankurah/proto';

// ─── RangeBound ──────────────────────────────────────────────────────────────

// Divergence: Rust enum RangeBound<T> → TS discriminated union [E8]
export type RangeBound<T> =
  | { type: 'Included'; value: T }
  | { type: 'Excluded'; value: T }
  | { type: 'Unbounded' };

// ─── Collatable ──────────────────────────────────────────────────────────────

// Divergence: Rust trait Collatable → TS interface [E8]
export interface Collatable {
  /// Convert the value to its binary representation for collation
  toBytes(): Uint8Array;

  /// Returns the immediate successor's binary representation if one exists
  successorBytes(): Uint8Array | null;

  /// Returns the immediate predecessor's binary representation if one exists
  predecessorBytes(): Uint8Array | null;

  /// Returns true if this value represents a minimum bound in its domain
  isMinimum(): boolean;

  /// Returns true if this value represents a maximum bound in its domain
  isMaximum(): boolean;
}

/// Compare two byte arrays lexicographically
function compareBytes(a: Uint8Array, b: Uint8Array): number {
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    if (a[i] < b[i]) return -1;
    if (a[i] > b[i]) return 1;
  }
  if (a.length < b.length) return -1;
  if (a.length > b.length) return 1;
  return 0;
}

/// Compare two Collatable values in the collation order
export function collatableCompare(a: Collatable, b: Collatable): number {
  return compareBytes(a.toBytes(), b.toBytes());
}

/// Check if a Collatable value is within a range
export function isInRange<T extends Collatable>(
  value: T,
  lower: RangeBound<T>,
  upper: RangeBound<T>,
): boolean {
  const cmpLower = (): number => {
    if (lower.type === 'Unbounded') return 1; // always passes
    return compareBytes(value.toBytes(), lower.value.toBytes());
  };
  const cmpUpper = (): number => {
    if (upper.type === 'Unbounded') return -1; // always passes
    return compareBytes(value.toBytes(), upper.value.toBytes());
  };

  switch (lower.type) {
    case 'Included': {
      const cl = cmpLower();
      if (cl < 0) return false; // value < lower
      break;
    }
    case 'Excluded': {
      const cl = cmpLower();
      if (cl <= 0) return false; // value <= lower
      break;
    }
    case 'Unbounded':
      break;
  }

  switch (upper.type) {
    case 'Included': {
      const cu = cmpUpper();
      if (cu > 0) return false; // value > upper
      break;
    }
    case 'Excluded': {
      const cu = cmpUpper();
      if (cu >= 0) return false; // value >= upper
      break;
    }
    case 'Unbounded':
      break;
  }

  return true;
}

// ─── Byte encoding helpers ───────────────────────────────────────────────────

function i16ToBeBytes(v: number): Uint8Array {
  const buf = new ArrayBuffer(2);
  new DataView(buf).setInt16(0, v, false);
  return new Uint8Array(buf);
}

function i32ToBeBytes(v: number): Uint8Array {
  const buf = new ArrayBuffer(4);
  new DataView(buf).setInt32(0, v, false);
  return new Uint8Array(buf);
}

function i64ToBeBytes(v: bigint): Uint8Array {
  const buf = new ArrayBuffer(8);
  new DataView(buf).setBigInt64(0, v, false);
  return new Uint8Array(buf);
}

function f64ToBeBytes(bits: bigint): Uint8Array {
  const buf = new ArrayBuffer(8);
  new DataView(buf).setBigUint64(0, bits, false);
  return new Uint8Array(buf);
}

function f64ToBits(v: number): bigint {
  const buf = new ArrayBuffer(8);
  new DataView(buf).setFloat64(0, v, false);
  return new DataView(buf).getBigUint64(0, false);
}

const I16_MAX = 32767;
const I16_MIN = -32768;
const I32_MAX = 2147483647;
const I32_MIN = -2147483648;
const I64_MAX = 9223372036854775807n;
const I64_MIN = -9223372036854775808n;
const U64_MAX = 0xFFFFFFFFFFFFFFFFn;
const SIGN_BIT = 1n << 63n;

// ─── impl Collatable for Literal ─────────────────────────────────────────────

function f64CollateBits(v: number): bigint {
  if (Number.isNaN(v)) {
    return U64_MAX; // NaN sorts last
  }
  const bits = f64ToBits(v);
  if (v >= 0) {
    return bits ^ SIGN_BIT; // Flip sign bit for positive numbers
  } else {
    return ~bits & U64_MAX; // Flip all bits for negative numbers (mask to 64-bit)
  }
}

/// Increment a byte array by 1 (returns null if all 0xFF)
function incrementBytes(bytes: Uint8Array): Uint8Array | null {
  const result = new Uint8Array(bytes);
  for (let i = result.length - 1; i >= 0; i--) {
    if (result[i] < 255) {
      result[i] += 1;
      for (let j = i + 1; j < result.length; j++) {
        result[j] = 0;
      }
      return result;
    }
  }
  return null;
}

/// Decrement a byte array by 1 (returns null if all 0x00)
function decrementBytes(bytes: Uint8Array): Uint8Array | null {
  const result = new Uint8Array(bytes);
  for (let i = result.length - 1; i >= 0; i--) {
    if (result[i] > 0) {
      result[i] -= 1;
      for (let j = i + 1; j < result.length; j++) {
        result[j] = 255;
      }
      return result;
    }
  }
  return null;
}

export function literalToBytes(lit: Literal): Uint8Array {
  return lit.match({
    String: (v) => new TextEncoder().encode(v.value),
    I16: (v) => i16ToBeBytes(v.value),
    I32: (v) => i32ToBeBytes(v.value),
    I64: (v) => i64ToBeBytes(v.value),
    F64: (v) => f64ToBeBytes(f64CollateBits(v.value)),
    Bool: (v) => new Uint8Array([v.value ? 1 : 0]),
    EntityId: (v) => new Uint8Array(v.value),
    Object: (v) => new Uint8Array(v.value),
    Binary: (v) => new Uint8Array(v.value),
    Json: (v) => {
      try {
        return new TextEncoder().encode(JSON.stringify(v.value));
      } catch {
        return new Uint8Array(0);
      }
    },
  });
}

export function literalSuccessorBytes(lit: Literal): Uint8Array | null {
  return lit.match({
    String: (v) => {
      const bytes = new TextEncoder().encode(v.value);
      // TODO - I think this is wrong. We shouldn't just push a byte. (mirrors Rust TODO)
      const result = new Uint8Array(bytes.length + 1);
      result.set(bytes);
      result[bytes.length] = 0;
      return result;
    },
    I16: (v) => v.value === I16_MAX ? null : i16ToBeBytes(v.value + 1),
    I32: (v) => v.value === I32_MAX ? null : i32ToBeBytes(v.value + 1),
    I64: (v) => v.value === I64_MAX ? null : i64ToBeBytes(v.value + 1n),
    F64: (v) => {
      if (Number.isNaN(v.value) || (v.value === Infinity)) return null;
      const bits = f64CollateBits(v.value);
      return f64ToBeBytes(bits + 1n);
    },
    Bool: (v) => v.value ? null : new Uint8Array([1]),
    EntityId: (v) => incrementBytes(v.value),
    Object: (v) => {
      const inc = incrementBytes(v.value);
      if (inc !== null) return inc;
      // All bytes are 255, append a zero byte
      const result = new Uint8Array(v.value.length + 1);
      result.set(v.value);
      result[v.value.length] = 0;
      return result;
    },
    Binary: (v) => {
      const inc = incrementBytes(v.value);
      if (inc !== null) return inc;
      // All bytes are 255, append a zero byte
      const result = new Uint8Array(v.value.length + 1);
      result.set(v.value);
      result[v.value.length] = 0;
      return result;
    },
    Json: () => null,
  });
}

export function literalPredecessorBytes(lit: Literal): Uint8Array | null {
  return lit.match({
    String: (v) => {
      if (v.value.length === 0) return null;
      const bytes = new TextEncoder().encode(v.value);
      return bytes.slice(0, bytes.length - 1);
    },
    I16: (v) => v.value === I16_MIN ? null : i16ToBeBytes(v.value - 1),
    I32: (v) => v.value === I32_MIN ? null : i32ToBeBytes(v.value - 1),
    I64: (v) => v.value === I64_MIN ? null : i64ToBeBytes(v.value - 1n),
    F64: (v) => {
      if (Number.isNaN(v.value) || (v.value === -Infinity)) return null;
      const bits = f64CollateBits(v.value);
      return f64ToBeBytes(bits - 1n);
    },
    Bool: (v) => v.value ? new Uint8Array([0]) : null,
    EntityId: (v) => decrementBytes(v.value),
    Object: (v) => {
      if (v.value.length === 0) return null;
      const dec = decrementBytes(v.value);
      if (dec !== null) return dec;
      // All bytes are 0, remove the last byte
      if (v.value.length > 1) {
        return v.value.slice(0, v.value.length - 1);
      }
      return null;
    },
    Binary: (v) => {
      if (v.value.length === 0) return null;
      const dec = decrementBytes(v.value);
      if (dec !== null) return dec;
      // All bytes are 0, remove the last byte
      if (v.value.length > 1) {
        return v.value.slice(0, v.value.length - 1);
      }
      return null;
    },
    Json: () => null,
  });
}

export function literalIsMinimum(lit: Literal): boolean {
  return lit.match({
    String: (v) => v.value.length === 0,
    I16: (v) => v.value === I16_MIN,
    I32: (v) => v.value === I32_MIN,
    I64: (v) => v.value === I64_MIN,
    F64: (v) => v.value === -Infinity,
    Bool: (v) => !v.value,
    EntityId: (v) => v.value.every((b) => b === 0),
    Object: (v) => v.value.length === 0,
    Binary: (v) => v.value.length === 0,
    Json: () => false,
  });
}

export function literalIsMaximum(lit: Literal): boolean {
  return lit.match({
    String: () => false, // Strings have no theoretical maximum
    I16: (v) => v.value === I16_MAX,
    I32: (v) => v.value === I32_MAX,
    I64: (v) => v.value === I64_MAX,
    F64: (v) => v.value === Infinity,
    Bool: (v) => v.value,
    EntityId: (v) => v.value.every((b) => b === 255),
    Object: () => false, // No theoretical maximum
    Binary: () => false, // No theoretical maximum
    Json: () => false,
  });
}

/// Wrap a Literal as a Collatable
export function literalCollatable(lit: Literal): Collatable {
  return {
    toBytes: () => literalToBytes(lit),
    successorBytes: () => literalSuccessorBytes(lit),
    predecessorBytes: () => literalPredecessorBytes(lit),
    isMinimum: () => literalIsMinimum(lit),
    isMaximum: () => literalIsMaximum(lit),
  };
}

// ─── impl Collatable for &str ────────────────────────────────────────────────

export function strToBytes(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

export function strSuccessorBytes(s: string): Uint8Array | null {
  if (s.length === 0) return null; // is_maximum returns false, but still no max for strings
  const bytes = strToBytes(s);
  const result = new Uint8Array(bytes.length + 1);
  result.set(bytes);
  result[bytes.length] = 0;
  return result;
}

export function strPredecessorBytes(s: string): Uint8Array | null {
  if (s.length === 0) return null;
  const bytes = strToBytes(s);
  return bytes.slice(0, bytes.length - 1);
}

export function strIsMinimum(s: string): boolean { return s.length === 0; }
export function strIsMaximum(_s: string): boolean { return false; }

// ─── impl Collatable for i64 ─────────────────────────────────────────────────

export function i64CollateToBytes(v: bigint): Uint8Array { return i64ToBeBytes(v); }
export function i64SuccessorBytes(v: bigint): Uint8Array | null { return v === I64_MAX ? null : i64ToBeBytes(v + 1n); }
export function i64PredecessorBytes(v: bigint): Uint8Array | null { return v === I64_MIN ? null : i64ToBeBytes(v - 1n); }
export function i64IsMinimum(v: bigint): boolean { return v === I64_MIN; }
export function i64IsMaximum(v: bigint): boolean { return v === I64_MAX; }

// ─── impl Collatable for f64 ─────────────────────────────────────────────────

export function f64CollateToBytes(v: number): Uint8Array { return f64ToBeBytes(f64CollateBits(v)); }

export function f64SuccessorBytes(v: number): Uint8Array | null {
  if (Number.isNaN(v) || (v === Infinity)) return null;
  const bits = f64CollateBits(v);
  return f64ToBeBytes(bits + 1n);
}

export function f64PredecessorBytes(v: number): Uint8Array | null {
  if (Number.isNaN(v) || (v === -Infinity)) return null;
  const bits = f64CollateBits(v);
  return f64ToBeBytes(bits - 1n);
}

export function f64IsMinimum(v: number): boolean { return v === -Infinity; }
export function f64IsMaximum(v: number): boolean { return v === Infinity; }

// ─── impl Collatable for EntityId ────────────────────────────────────────────

export function entityIdCollateToBytes(id: EntityId): Uint8Array { return id.toBytes(); }

export function entityIdSuccessorBytes(id: EntityId): Uint8Array | null {
  const bytes = id.toBytes();
  if (bytes.every((b) => b === 255)) return null;
  return incrementBytes(bytes);
}

export function entityIdPredecessorBytes(id: EntityId): Uint8Array | null {
  const bytes = id.toBytes();
  if (bytes.every((b) => b === 0)) return null;
  return decrementBytes(bytes);
}

export function entityIdIsMinimum(id: EntityId): boolean { return id.toBytes().every((b) => b === 0); }
export function entityIdIsMaximum(id: EntityId): boolean { return id.toBytes().every((b) => b === 255); }
