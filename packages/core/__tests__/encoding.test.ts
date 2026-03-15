// MIRRORS: ankurah/core/src/indexing/encoding.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { encodeComponentTyped } from '../src/indexing/encoding.ts';
import { ValueType } from '../src/value/index.ts';
import type { Value } from '../src/value/index.ts';

// ── Byte array comparison helper ─────────────────────────────────────

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

describe('encoding', () => {
  // Rust: fn test_desc_ordering()
  test('desc ordering', () => {
    const a = encodeComponentTyped(
      { type: 'String', value: 'a' } as Value,
      ValueType.String,
      true,
    );
    const b = encodeComponentTyped(
      { type: 'String', value: 'b' } as Value,
      ValueType.String,
      true,
    );

    // DESC: "a" should sort after "b" (reversed)
    expect(compareBytes(a, b)).toBe(1);
  });

  // Rust: fn test_asc_ordering()
  test('asc ordering', () => {
    const a = encodeComponentTyped(
      { type: 'String', value: 'a' } as Value,
      ValueType.String,
      false,
    );
    const b = encodeComponentTyped(
      { type: 'String', value: 'b' } as Value,
      ValueType.String,
      false,
    );

    // ASC: "a" should sort before "b"
    expect(compareBytes(a, b)).toBe(-1);
  });
});
