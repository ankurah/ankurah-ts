// MIRRORS: ankurah/core/src/collation.rs #[cfg(test)] mod tests

import { describe, test, expect } from 'bun:test';
import { Literal } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';
import {
  type Collatable,
  isInRange,
  strToBytes,
  strSuccessorBytes,
  strPredecessorBytes,
  strIsMinimum,
  strIsMaximum,
  i64CollateToBytes,
  i64SuccessorBytes,
  i64PredecessorBytes,
  i64IsMinimum,
  i64IsMaximum,
  f64CollateToBytes,
  f64SuccessorBytes,
  f64PredecessorBytes,
  f64IsMinimum,
  f64IsMaximum,
  literalToBytes,
  literalSuccessorBytes,
  literalPredecessorBytes,
  literalIsMinimum,
  literalIsMaximum,
} from './collation.ts';

// ── Helpers ──

/** Compare two Uint8Arrays lexicographically: negative if a < b, 0 if equal, positive if a > b */
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

/** Wrap a bigint as Collatable (mirrors Rust impl Collatable for i64) */
function i64Collatable(v: bigint): Collatable {
  return {
    toBytes: () => i64CollateToBytes(v),
    successorBytes: () => i64SuccessorBytes(v),
    predecessorBytes: () => i64PredecessorBytes(v),
    isMinimum: () => i64IsMinimum(v),
    isMaximum: () => i64IsMaximum(v),
  };
}

/** Read an i64 from big-endian bytes (mirrors Rust i64::from_be_bytes) */
function i64FromBeBytes(bytes: Uint8Array): bigint {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return view.getBigInt64(0, false);
}

// ── Tests ──

describe('collation', () => {
  // Rust: fn test_string_collation()
  test('test_string_collation', () => {
    const s = 'hello';
    const sBytes = strToBytes(s);
    const succ = strSuccessorBytes(s);
    const pred = strPredecessorBytes(s);

    expect(succ).not.toBeNull();
    expect(compareBytes(succ!, sBytes)).toBeGreaterThan(0);

    expect(pred).not.toBeNull();
    expect(compareBytes(pred!, sBytes)).toBeLessThan(0);

    expect(strIsMinimum(s)).toBe(false);
    expect(strIsMaximum(s)).toBe(false);

    const empty = '';
    expect(strIsMinimum(empty)).toBe(true);
    expect(strPredecessorBytes(empty)).toBeNull();
  });

  // Rust: fn test_integer_collation()
  test('test_integer_collation', () => {
    const n = 42n;
    const succBytes = i64SuccessorBytes(n);
    const predBytes = i64PredecessorBytes(n);

    expect(succBytes).not.toBeNull();
    expect(i64FromBeBytes(succBytes!)).toBe(43n);

    expect(predBytes).not.toBeNull();
    expect(i64FromBeBytes(predBytes!)).toBe(41n);

    expect(i64IsMinimum(n)).toBe(false);
    expect(i64IsMaximum(n)).toBe(false);

    const I64_MAX = 9223372036854775807n;
    const I64_MIN = -9223372036854775808n;

    expect(i64SuccessorBytes(I64_MAX)).toBeNull();
    expect(i64PredecessorBytes(I64_MIN)).toBeNull();
    expect(i64IsMaximum(I64_MAX)).toBe(true);
    expect(i64IsMinimum(I64_MIN)).toBe(true);
  });

  // Rust: fn test_float_collation()
  test('test_float_collation', () => {
    const f = 1.0;
    const fBytes = f64CollateToBytes(f);
    const succ = f64SuccessorBytes(f);
    const pred = f64PredecessorBytes(f);

    expect(succ).not.toBeNull();
    expect(compareBytes(succ!, fBytes)).toBeGreaterThan(0);

    expect(pred).not.toBeNull();
    expect(compareBytes(pred!, fBytes)).toBeLessThan(0);

    expect(f64IsMinimum(f)).toBe(false);
    expect(f64IsMaximum(f)).toBe(false);

    expect(f64IsMaximum(Infinity)).toBe(true);
    expect(f64IsMinimum(-Infinity)).toBe(true);
    expect(f64SuccessorBytes(Infinity)).toBeNull();
    expect(f64PredecessorBytes(-Infinity)).toBeNull();

    const nan = NaN;
    expect(f64SuccessorBytes(nan)).toBeNull();
    expect(f64PredecessorBytes(nan)).toBeNull();
  });

  // Rust: fn test_range_bounds()
  test('test_range_bounds', () => {
    const n = i64Collatable(42n);

    // Test inclusive bounds
    expect(isInRange(n,
      { type: 'Included', value: i64Collatable(40n) },
      { type: 'Included', value: i64Collatable(45n) },
    )).toBe(true);
    expect(isInRange(n,
      { type: 'Included', value: i64Collatable(42n) },
      { type: 'Included', value: i64Collatable(45n) },
    )).toBe(true);
    expect(isInRange(n,
      { type: 'Included', value: i64Collatable(40n) },
      { type: 'Included', value: i64Collatable(42n) },
    )).toBe(true);

    // Test exclusive bounds
    expect(isInRange(n,
      { type: 'Excluded', value: i64Collatable(40n) },
      { type: 'Excluded', value: i64Collatable(43n) },
    )).toBe(true);
    expect(isInRange(n,
      { type: 'Excluded', value: i64Collatable(42n) },
      { type: 'Excluded', value: i64Collatable(43n) },
    )).toBe(false);

    // Test mixed bounds
    expect(isInRange(n,
      { type: 'Included', value: i64Collatable(42n) },
      { type: 'Excluded', value: i64Collatable(43n) },
    )).toBe(true);
    expect(isInRange(n,
      { type: 'Excluded', value: i64Collatable(41n) },
      { type: 'Excluded', value: i64Collatable(42n) },
    )).toBe(false);

    // Test unbounded
    expect(isInRange(n,
      { type: 'Unbounded' },
      { type: 'Included', value: i64Collatable(45n) },
    )).toBe(true);
    expect(isInRange(n,
      { type: 'Included', value: i64Collatable(40n) },
      { type: 'Unbounded' },
    )).toBe(true);
    expect(isInRange(n,
      { type: 'Unbounded' },
      { type: 'Unbounded' },
    )).toBe(true);
  });

  // Rust: fn test_literal_i16_collation()
  test('test_literal_i16_collation', () => {
    const lit = Literal.I16(100);
    const litBytes = literalToBytes(lit);
    const succ = literalSuccessorBytes(lit);
    const pred = literalPredecessorBytes(lit);

    expect(succ).not.toBeNull();
    expect(compareBytes(succ!, litBytes)).toBeGreaterThan(0);

    expect(pred).not.toBeNull();
    expect(compareBytes(pred!, litBytes)).toBeLessThan(0);

    expect(literalIsMinimum(lit)).toBe(false);
    expect(literalIsMaximum(lit)).toBe(false);

    const maxLit = Literal.I16(32767); // i16::MAX
    const minLit = Literal.I16(-32768); // i16::MIN
    expect(literalSuccessorBytes(maxLit)).toBeNull();
    expect(literalPredecessorBytes(minLit)).toBeNull();
    expect(literalIsMaximum(maxLit)).toBe(true);
    expect(literalIsMinimum(minLit)).toBe(true);
  });

  // Rust: fn test_literal_i32_collation()
  test('test_literal_i32_collation', () => {
    const lit = Literal.I32(1000);
    const litBytes = literalToBytes(lit);
    const succ = literalSuccessorBytes(lit);
    const pred = literalPredecessorBytes(lit);

    expect(succ).not.toBeNull();
    expect(compareBytes(succ!, litBytes)).toBeGreaterThan(0);

    expect(pred).not.toBeNull();
    expect(compareBytes(pred!, litBytes)).toBeLessThan(0);

    expect(literalIsMinimum(lit)).toBe(false);
    expect(literalIsMaximum(lit)).toBe(false);

    const maxLit = Literal.I32(2147483647); // i32::MAX
    const minLit = Literal.I32(-2147483648); // i32::MIN
    expect(literalSuccessorBytes(maxLit)).toBeNull();
    expect(literalPredecessorBytes(minLit)).toBeNull();
    expect(literalIsMaximum(maxLit)).toBe(true);
    expect(literalIsMinimum(minLit)).toBe(true);
  });

  // Rust: fn test_literal_entity_id_collation()
  test('test_literal_entity_id_collation', () => {
    // Divergence: Rust uses Ulid::new(); TS uses EntityId.new() then extracts bytes
    const entityId = EntityId.new();
    const lit = Literal.EntityId(entityId.toBytes());

    // Test basic operations
    expect(literalIsMinimum(lit)).toBe(false);
    expect(literalIsMaximum(lit)).toBe(false);

    // Test minimum ULID (all zeros)
    const minLit = Literal.EntityId(new Uint8Array(16)); // all zeros
    expect(literalIsMinimum(minLit)).toBe(true);
    expect(literalPredecessorBytes(minLit)).toBeNull();

    // Test maximum ULID (all 255s)
    const maxBytes = new Uint8Array(16);
    maxBytes.fill(255);
    const maxLit = Literal.EntityId(maxBytes);
    expect(literalIsMaximum(maxLit)).toBe(true);
    expect(literalSuccessorBytes(maxLit)).toBeNull();
  });

  // Rust: fn test_literal_binary_collation()
  test('test_literal_binary_collation', () => {
    const lit = Literal.Binary(new Uint8Array([1, 2, 3]));
    const litBytes = literalToBytes(lit);
    const succ = literalSuccessorBytes(lit);
    const pred = literalPredecessorBytes(lit);

    expect(succ).not.toBeNull();
    expect(compareBytes(succ!, litBytes)).toBeGreaterThan(0);

    expect(pred).not.toBeNull();
    expect(compareBytes(pred!, litBytes)).toBeLessThan(0);

    expect(literalIsMinimum(lit)).toBe(false);
    expect(literalIsMaximum(lit)).toBe(false);

    const emptyLit = Literal.Binary(new Uint8Array([]));
    expect(literalIsMinimum(emptyLit)).toBe(true);
    expect(literalPredecessorBytes(emptyLit)).toBeNull();
    expect(literalIsMaximum(emptyLit)).toBe(false);
  });

  // Rust: fn test_literal_object_collation()
  test('test_literal_object_collation', () => {
    const lit = Literal.Object(new Uint8Array([10, 20, 30]));
    const litBytes = literalToBytes(lit);
    const succ = literalSuccessorBytes(lit);
    const pred = literalPredecessorBytes(lit);

    expect(succ).not.toBeNull();
    expect(compareBytes(succ!, litBytes)).toBeGreaterThan(0);

    expect(pred).not.toBeNull();
    expect(compareBytes(pred!, litBytes)).toBeLessThan(0);

    expect(literalIsMinimum(lit)).toBe(false);
    expect(literalIsMaximum(lit)).toBe(false);

    const emptyLit = Literal.Object(new Uint8Array([]));
    expect(literalIsMinimum(emptyLit)).toBe(true);
    expect(literalPredecessorBytes(emptyLit)).toBeNull();
    expect(literalIsMaximum(emptyLit)).toBe(false);
  });
});
