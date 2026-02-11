// MIRRORS: ankurah/core/src/indexing/encoding.rs
//
// Stub implementation: uses basic collation encoding from collatable.ts.
// The full collation-safe encoding (0x00 escaping, sign-flipped big-endian, etc.)
// can be refined later. For now:
// - For each value + keypart, convert value to collation bytes via valueCollatableToBytes()
// - If direction is Desc, flip all bytes (XOR 0xFF)
// - Concatenate all byte arrays

import type { Value } from '../value/index.ts';
import { ValueType } from '../value/index.ts';
import { valueCollatableToBytes } from '../value/collatable.ts';
import { IndexDirection, type KeySpec } from './key_spec.ts';

// ── IndexError ───────────────────────────────────────────────────────
// Rust: `pub enum IndexError { TypeMismatch(ValueType, ValueType) }`

export class IndexError extends Error {
  readonly expected: ValueType;
  readonly got: ValueType;

  constructor(expected: ValueType, got: ValueType) {
    super(`Type mismatch: expected ${expected}, got ${got}`);
    this.name = 'IndexError';
    this.expected = expected;
    this.got = got;
  }

  static typeMismatch(expected: ValueType, got: ValueType): IndexError {
    return new IndexError(expected, got);
  }
}

/**
 * Type-aware encoding using KeySpec for validation and optimization.
 * Stub implementation using collation bytes with desc byte-flipping.
 *
 * Rust: `pub fn encode_tuple_values_with_key_spec(values: &[Value], key_spec: &KeySpec) -> Result<Vec<u8>, IndexError>`
 */
export function encodeTupleValuesWithKeySpec(values: Value[], keySpec: KeySpec): Uint8Array {
  const parts: Uint8Array[] = [];
  const len = Math.min(values.length, keySpec.keyparts.length);

  for (let i = 0; i < len; i++) {
    const value = values[i];
    const keypart = keySpec.keyparts[i];

    // Convert value to collation bytes
    const bytes = valueCollatableToBytes(value);

    if (keypart.direction === IndexDirection.Desc) {
      // Flip all bytes (XOR 0xFF) for descending order
      const flipped = new Uint8Array(bytes.length);
      for (let j = 0; j < bytes.length; j++) {
        flipped[j] = bytes[j] ^ 0xFF;
      }
      parts.push(flipped);
    } else {
      parts.push(bytes);
    }
  }

  // Concatenate all byte arrays
  let totalLen = 0;
  for (const p of parts) {
    totalLen += p.length;
  }
  const result = new Uint8Array(totalLen);
  let offset = 0;
  for (const p of parts) {
    result.set(p, offset);
    offset += p.length;
  }
  return result;
}
