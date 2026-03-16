// MIRRORS: ankurah/storage/sqlite/src/value.rs

import type { Value } from '@ankurah/core';

/**
 * SQLite value wrapper for type mapping.
 *
 * Rust: `pub enum SqliteValue { Text, Integer, Real, Blob, Jsonb, Null }`
 * Divergence: TS uses a discriminated union instead of Enum<V> since this is
 * a storage-layer type that maps to SQLite driver values, not a protocol type [E16].
 */
export type SqliteValue =
  | { type: 'Text'; value: string }
  | { type: 'Integer'; value: number }
  | { type: 'Real'; value: number }
  | { type: 'Blob'; value: Uint8Array }
  | { type: 'Jsonb'; value: unknown }
  | { type: 'Null' };

/** Get the SQLite type name for column creation. */
export function sqliteValueType(v: SqliteValue): string {
  switch (v.type) {
    case 'Text': return 'TEXT';
    case 'Integer': return 'INTEGER';
    case 'Real': return 'REAL';
    case 'Blob': return 'BLOB';
    case 'Jsonb': return 'BLOB'; // JSONB stored as BLOB, queried via jsonb()
    case 'Null': return 'TEXT';  // Default to TEXT for NULL
  }
}

/** Check if this value is a JSONB type that needs special SQL handling. */
export function sqliteValueIsJsonb(v: SqliteValue): boolean {
  return v.type === 'Jsonb';
}

/** Get the JSON string representation (for use with jsonb() function). */
export function sqliteValueAsJsonString(v: SqliteValue): string | null {
  if (v.type === 'Jsonb') {
    return JSON.stringify(v.value);
  }
  return null;
}

/**
 * Convert to a raw SQL parameter value.
 * Note: For JSONB, this returns the JSON text — the caller must wrap with jsonb().
 *
 * Divergence: Returns a plain JS value suitable for SQLite driver bind params [E16].
 */
export function sqliteValueToParam(v: SqliteValue): string | number | Uint8Array | null {
  switch (v.type) {
    case 'Text': return v.value;
    case 'Integer': return v.value;
    case 'Real': return v.value;
    case 'Blob': return v.value;
    case 'Jsonb': return JSON.stringify(v.value);
    case 'Null': return null;
  }
}

/** Convert an ankurah Value to a SqliteValue. Rust: `impl From<Value> for SqliteValue` */
export function sqliteValueFromValue(value: Value): SqliteValue {
  switch (value.type) {
    case 'String': return { type: 'Text', value: value.value };
    case 'I16': return { type: 'Integer', value: value.value };
    case 'I32': return { type: 'Integer', value: value.value };
    case 'I64': return { type: 'Integer', value: Number(value.value) };
    case 'F64': return { type: 'Real', value: value.value };
    case 'Bool': return { type: 'Integer', value: value.value ? 1 : 0 };
    case 'EntityId': return { type: 'Text', value: value.value.toBase64() };
    case 'Object': return { type: 'Blob', value: value.value };
    case 'Binary': return { type: 'Blob', value: value.value };
    case 'Json': return { type: 'Jsonb', value: value.value };
  }
}

/** Convert an optional Value to a SqliteValue. Rust: `impl From<Option<Value>> for SqliteValue` */
export function sqliteValueFromOptionalValue(value: Value | null): SqliteValue {
  if (value === null) return { type: 'Null' };
  return sqliteValueFromValue(value);
}
