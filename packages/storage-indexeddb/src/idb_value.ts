// MIRRORS: ankurah/storage/indexeddb-wasm/src/idb_value.rs

// Divergence: Rust wraps Value in an IdbValue newtype with From/Into impls. [E16]
// In TS, we export conversion functions directly since there's no WASM boundary.

import { EntityId } from '@ankurah/proto';
import type { Value } from '@ankurah/core';

// Divergence: IdbKey is a DOM type not available in all TS configs. [E16]
// Define locally to avoid requiring DOM lib at the root tsconfig level.
type IdbKey = number | string | ArrayBuffer | IdbKey[];

/// Convert boolean values to 0/1 numbers recursively in a JSON structure.
/// IndexedDB doesn't support boolean keys, so we must encode bools as numbers
/// for subpath indexing to work (e.g., `data.enabled = true` → `data.enabled = 1`).
function convertJsonBoolsToNumbers(json: unknown): unknown {
  if (typeof json === 'boolean') {
    return json ? 1 : 0;
  }
  if (Array.isArray(json)) {
    return json.map(convertJsonBoolsToNumbers);
  }
  if (json !== null && typeof json === 'object') {
    const result: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(json as Record<string, unknown>)) {
      result[k] = convertJsonBoolsToNumbers(v);
    }
    return result;
  }
  return json;
}

/// Maximum safe integer in JavaScript (2^53 - 1)
export const MAX_SAFE_INTEGER = 9_007_199_254_740_991;

/// Minimum safe integer in JavaScript (-(2^53 - 1))
export const MIN_SAFE_INTEGER = -9_007_199_254_740_991;

/// Convert a Value to an IndexedDB-compatible JS value
///
/// This encoding ensures values can be used both as:
/// - Field values stored in IndexedDB objects
/// - Index keys for range queries and compound indexes
/// - Prefix guards during cursor iteration
///
/// Special handling for i64:
/// - Negative values: always stored as number (accept truncation beyond ±2^53)
/// - Positive values 0..=2^53-1: stored as number (efficient)
/// - Positive values >2^53-1: stored as zero-padded string (full precision)
export function valueToIdb(value: Value): IdbKey {
  switch (value.type) {
    case 'I16':
      return value.value;
    case 'I32':
      return value.value;
    case 'I64': {
      const x = value.value;
      if (x < 0) {
        // Negative: always use number
        if (x < MIN_SAFE_INTEGER) {
          console.warn(`Negative i64 ${x} exceeds safe integer range (${MIN_SAFE_INTEGER}), precision loss will occur`);
        }
        return x;
      } else if (x <= MAX_SAFE_INTEGER) {
        // Positive safe range: use number
        return x;
      } else {
        // Positive beyond safe range: use zero-padded string
        // i64 max is 9223372036854775807 (19 digits), pad to 20
        // All strings are lexicographically after all numbers in IndexedDB keys
        // so we can use this to our advantage as long as we do it consistently
        return String(x).padStart(20, '0');
      }
    }
    case 'F64':
      return value.value;
    case 'Bool':
      // IndexedDB keys don't support boolean
      return value.value ? 1 : 0;
    case 'String':
      return value.value;
    case 'EntityId':
      return value.value.toBase64();
    case 'Binary':
    case 'Object':
      // Divergence: Rust uses Uint8Array→JsValue via wasm-bindgen. [E16]
      // TS uses ArrayBuffer directly for IndexedDB key compatibility.
      return value.value.buffer.slice(
        value.value.byteOffset,
        value.value.byteOffset + value.value.byteLength,
      ) as ArrayBuffer;
    case 'Json': {
      // Json is stored as a parsed JS object to enable IndexedDB's native nested property indexing.
      // IMPORTANT: Booleans must be converted to 0/1 because IDB doesn't support boolean keys.
      return convertJsonBoolsToNumbers(value.value) as IdbKey;
    }
  }
}

/// Convert from a JS value retrieved from IndexedDB back to a Value
///
/// Uses standard Value conversion without schema information. Type information may be lost:
/// - 0/1 numbers → I32 (bool type info lost)
/// - Zero-padded numeric strings → String (i64 type info lost for large values)
/// - JS objects → Value::Json (serialized back to JSON)
///
/// **Future enhancement:** Accept schema/ValueType hints for direct conversion to proper types.
///
/// **Current workaround:** We rely on Value-to-Value casting in predicate comparisons
/// (see `compare_values_with_cast` in `filter.ts`). When comparing values from IndexedDB
/// against query literals, the casting system automatically converts:
/// - `Value::I32(1)` ↔ `Value::Bool(true)`
/// - `Value::String("9007199254740992000")` ↔ `Value::I64(9007199254740992000)`
export function idbToValue(jsValue: unknown): Value {
  if (typeof jsValue === 'number') {
    if (Number.isInteger(jsValue)) {
      return { type: 'I32', value: jsValue };
    }
    return { type: 'F64', value: jsValue };
  }

  if (typeof jsValue === 'string') {
    return { type: 'String', value: jsValue };
  }

  if (typeof jsValue === 'boolean') {
    return { type: 'Bool', value: jsValue };
  }

  if (jsValue instanceof ArrayBuffer) {
    return { type: 'Binary', value: new Uint8Array(jsValue) };
  }

  if (jsValue instanceof Uint8Array) {
    return { type: 'Binary', value: jsValue };
  }

  // Object — try to treat as JSON
  if (jsValue !== null && typeof jsValue === 'object') {
    return { type: 'Json', value: jsValue };
  }

  // Fallback
  return { type: 'Json', value: jsValue };
}
