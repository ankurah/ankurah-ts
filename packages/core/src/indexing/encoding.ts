// MIRRORS: ankurah/core/src/indexing/encoding.rs

import type { Value } from '../value/index.ts';
import { ValueType, valueType } from '../value/index.ts';
import { castTo } from '../value/cast.ts';
import { valueCollatableToBytes } from '../value/collatable.ts';
import { type KeySpec, isDesc } from './key_spec.ts';

// ── IndexError ───────────────────────────────────────────────────────
// Rust: `pub enum IndexError { TypeMismatch(ValueType, ValueType) }`
// Divergence: Error subclass, not Enum — per rule A8 [E8].

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

// ── JSON type tags ───────────────────────────────────────────────────
// Chosen to provide sensible sort order: null < bool < int < float < string
// Each type uses fixed-width encoding where possible to avoid sentinel issues.

const JSON_TAG_NULL = 0x00;
const JSON_TAG_BOOL = 0x10;
const JSON_TAG_INT = 0x20;   // i64: fixed 8 bytes, no sentinel needed
const JSON_TAG_FLOAT = 0x30; // f64: fixed 8 bytes, no sentinel needed
const JSON_TAG_STRING = 0x40; // variable length, uses 0x00 sentinel with 0x00→0x00 0xFF escaping

// ── encode_component_typed ───────────────────────────────────────────
// Rust: pub fn encode_component_typed(value, expected_type, descending) -> Result<Vec<u8>, IndexError>

export function encodeComponentTyped(
  value: Value,
  expectedType: ValueType,
  descending: boolean,
): Uint8Array {
  // Cast value to expected type (short-circuits if types already match)
  let castValue: Value;
  try {
    castValue = castTo(value, expectedType);
  } catch {
    throw IndexError.typeMismatch(expectedType, valueType(value));
  }

  return encodeValueComponent(castValue, expectedType, descending);
}

// ── encode_value_component ───────────────────────────────────────────
// Rust: fn encode_value_component(value, expected_type, descending) -> Result<Vec<u8>, IndexError>

function encodeValueComponent(
  value: Value,
  expectedType: ValueType,
  descending: boolean,
): Uint8Array {
  switch (value.type) {
    case 'String': {
      if (expectedType !== ValueType.String) break;
      const bytes = new TextEncoder().encode(value.value);
      if (!descending) {
        // ASC: [escaped UTF-8][0x00] — no type tag needed
        const out: number[] = [];
        for (const b of bytes) {
          if (b === 0x00) {
            out.push(0x00, 0xFF);
          } else {
            out.push(b);
          }
        }
        out.push(0x00); // terminator
        return new Uint8Array(out);
      } else {
        // DESC: [inv(payload) with 0xFF escaped as 0xFF 0x00][0xFF 0xFF]
        const out: number[] = [];
        for (const b of bytes) {
          const inv = (0xFF - b) & 0xFF;
          if (inv === 0xFF) {
            out.push(0xFF, 0x00);
          } else {
            out.push(inv);
          }
        }
        out.push(0xFF, 0xFF); // terminator
        return new Uint8Array(out);
      }
    }

    case 'I16':
    case 'I32':
    case 'I64': {
      if (expectedType !== ValueType.I16 && expectedType !== ValueType.I32 && expectedType !== ValueType.I64) break;
      // Integers are encoded big-endian (order-preserving). DESC: invert payload bytes.
      const bytes = valueCollatableToBytes(value);
      if (!descending) {
        return bytes;
      } else {
        const out = new Uint8Array(bytes.length);
        for (let i = 0; i < bytes.length; i++) {
          out[i] = (0xFF - bytes[i]) & 0xFF;
        }
        return out;
      }
    }

    case 'F64': {
      if (expectedType !== ValueType.F64) break;
      // F64 uses collation ordering (NaN sorts last, proper IEEE 754 ordering). DESC: invert payload bytes.
      const bytes = valueCollatableToBytes(value);
      if (!descending) {
        return bytes;
      } else {
        const out = new Uint8Array(bytes.length);
        for (let i = 0; i < bytes.length; i++) {
          out[i] = (0xFF - bytes[i]) & 0xFF;
        }
        return out;
      }
    }

    case 'Bool': {
      if (expectedType !== ValueType.Bool) break;
      // ASC: false(0) < true(1). DESC: invert payload to flip order.
      const b = valueCollatableToBytes(value)[0];
      return new Uint8Array([!descending ? b : (0xFF - b) & 0xFF]);
    }

    case 'EntityId': {
      if (expectedType !== ValueType.EntityId) break;
      // Fixed-width EntityId: no terminator needed
      const bytes = value.value.toBytes();
      if (!descending) {
        return new Uint8Array(bytes);
      } else {
        const out = new Uint8Array(bytes.length);
        for (let i = 0; i < bytes.length; i++) {
          out[i] = (0xFF - bytes[i]) & 0xFF;
        }
        return out;
      }
    }

    case 'Object':
    case 'Binary': {
      if (expectedType !== ValueType.Binary && expectedType !== ValueType.Object) break;
      const bytes = value.value;
      if (!descending) {
        // ASC: [escaped bytes][0x00] — terminator needed for variable-width
        const out: number[] = [];
        for (const b of bytes) {
          if (b === 0x00) {
            out.push(0x00, 0xFF);
          } else {
            out.push(b);
          }
        }
        out.push(0x00); // terminator
        return new Uint8Array(out);
      } else {
        // DESC: [inv(bytes) with 0xFF escaped as 0xFF 0x00][0xFF 0xFF]
        const out: number[] = [];
        for (const b of bytes) {
          const inv = (0xFF - b) & 0xFF;
          if (inv === 0xFF) {
            out.push(0xFF, 0x00);
          } else {
            out.push(inv);
          }
        }
        out.push(0xFF, 0xFF); // terminator
        return new Uint8Array(out);
      }
    }

    case 'Json': {
      if (expectedType !== ValueType.Json) break;
      // JSON: type-tagged encoding preserving original type (no cross-type casting)
      return encodeJsonValue(value.value, descending);
    }
  }

  // Fallthrough: type mismatch
  throw IndexError.typeMismatch(expectedType, valueType(value));
}

// ── encode_json_value ────────────────────────────────────────────────
// Rust: fn encode_json_value(json, descending) -> Vec<u8>
// Different types get different prefixes, so "9" (string) != 9 (int).

function encodeJsonValue(json: unknown, descending: boolean): Uint8Array {
  let tag: number;
  let payload: Uint8Array;

  if (json === null || json === undefined) {
    tag = JSON_TAG_NULL;
    payload = new Uint8Array(0);
  } else if (typeof json === 'boolean') {
    tag = JSON_TAG_BOOL;
    payload = new Uint8Array([json ? 1 : 0]);
  } else if (typeof json === 'number') {
    if (Number.isInteger(json)) {
      // i64: fixed 8 bytes big-endian with sign flip for proper ordering
      tag = JSON_TAG_INT;
      payload = valueCollatableToBytes({ type: 'I64', value: json });
    } else {
      // f64: fixed 8 bytes with IEEE 754 ordering
      tag = JSON_TAG_FLOAT;
      payload = valueCollatableToBytes({ type: 'F64', value: json });
    }
  } else if (typeof json === 'string') {
    // Variable length: escape 0x00 bytes and add 0x00 terminator
    tag = JSON_TAG_STRING;
    const bytes = new TextEncoder().encode(json);
    const parts: number[] = [];
    for (const b of bytes) {
      if (b === 0x00) {
        parts.push(0x00, 0xFF);
      } else {
        parts.push(b);
      }
    }
    parts.push(0x00); // terminator
    payload = new Uint8Array(parts);
  } else {
    // Objects and arrays are unsortable — encode as null
    tag = JSON_TAG_NULL;
    payload = new Uint8Array(0);
  }

  if (!descending) {
    const out = new Uint8Array(1 + payload.length);
    out[0] = tag;
    out.set(payload, 1);
    return out;
  } else {
    // DESC: invert tag and all payload bytes
    const out = new Uint8Array(1 + payload.length);
    out[0] = (0xFF - tag) & 0xFF;
    for (let i = 0; i < payload.length; i++) {
      out[1 + i] = (0xFF - payload[i]) & 0xFF;
    }
    return out;
  }
}

// ── encode_tuple_values_with_key_spec ────────────────────────────────
// Rust: pub fn encode_tuple_values_with_key_spec(values, key_spec) -> Result<Vec<u8>, IndexError>
// TODO: Add NULL handling later

export function encodeTupleValuesWithKeySpec(values: Value[], keySpec: KeySpec): Uint8Array {
  const parts: Uint8Array[] = [];
  const len = Math.min(values.length, keySpec.keyparts.length);

  for (let i = 0; i < len; i++) {
    const value = values[i];
    const keypart = keySpec.keyparts[i];

    // Use type-aware encoding without type tags
    const bytes = encodeComponentTyped(value, keypart.valueType, isDesc(keypart.direction));
    parts.push(bytes);
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
