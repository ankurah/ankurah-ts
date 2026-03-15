// MIRRORS: ankurah/core/src/value/mod.rs

import { EntityId } from '@ankurah/proto';
import type { Literal } from '@ankurah/ankql';
import { PropertyError } from '../property/traits.ts';

// Re-export sub-modules (matching Rust mod.rs pub use / pub mod)
export type { CastError } from './cast';
export { CastErrorException, castTo, tryCastTo } from './cast';
export type { CollectionSchema } from './cast_predicate';
export { castPredicateTypes } from './cast_predicate';
export { valueCollatableToBytes, valueCollatableSuccessorBytes, valueCollatablePredecessorBytes, valueCollatableIsMinimum, valueCollatableIsMaximum, valueCollatableCompare } from './collatable';

// ── Value ────────────────────────────────────────────────────────────
// Discriminated union matching Rust Value enum variants.

export type Value =
  | { type: 'I16'; value: number }
  | { type: 'I32'; value: number }
  | { type: 'I64'; value: number }
  | { type: 'F64'; value: number }
  | { type: 'Bool'; value: boolean }
  | { type: 'String'; value: string }
  | { type: 'EntityId'; value: EntityId }
  | { type: 'Object'; value: Uint8Array }
  | { type: 'Binary'; value: Uint8Array }
  | { type: 'Json'; value: unknown };

// ── ValueType ────────────────────────────────────────────────────────
// Discriminant enum for Value (matches Rust ValueType enum).

export enum ValueType {
  I16 = 'I16',
  I32 = 'I32',
  I64 = 'I64',
  F64 = 'F64',
  Bool = 'Bool',
  String = 'String',
  EntityId = 'EntityId',
  Object = 'Object',
  Binary = 'Binary',
  Json = 'Json',
}

/** Get the ValueType discriminant for a Value. Mirrors Rust ValueType::of(). */
export function valueType(v: Value): ValueType {
  switch (v.type) {
    case 'I16': return ValueType.I16;
    case 'I32': return ValueType.I32;
    case 'I64': return ValueType.I64;
    case 'F64': return ValueType.F64;
    case 'Bool': return ValueType.Bool;
    case 'String': return ValueType.String;
    case 'EntityId': return ValueType.EntityId;
    case 'Object': return ValueType.Object;
    case 'Binary': return ValueType.Binary;
    case 'Json': return ValueType.Json;
  }
}

// ── Value factory ────────────────────────────────────────────────────

/** Create a Json Value from any JSON-serializable value. Mirrors Rust Value::json(). */
export function valueJson(v: unknown): Value {
  return { type: 'Json', value: v };
}

/**
 * Parse this value as JSON into a plain object.
 * Works for Json, Object, Binary (as bytes) and String variants.
 * Throws PropertyError for numeric, bool, and EntityId types.
 * Mirrors Rust Value::parse_as_json().
 */
export function parseAsJson(value: Value): unknown {
  switch (value.type) {
    case 'Json':
      // Rust: serde_json::from_value(json.clone()) — round-trip through JSON to get a plain object
      return JSON.parse(JSON.stringify(value.value));
    case 'Object':
    case 'Binary': {
      const text = new TextDecoder().decode(value.value);
      return JSON.parse(text);
    }
    case 'String':
      return JSON.parse(value.value);
    default:
      throw PropertyError.invalidVariant(value, 'JSON');
  }
}

/**
 * Parse this value as a string.
 * Only works for Value::String variant.
 * Throws PropertyError for other types.
 * Mirrors Rust Value::parse_as_string().
 */
export function parseAsString(value: Value): string {
  if (value.type === 'String') {
    return value.value;
  }
  throw PropertyError.invalidVariant(value, 'string');
}

// ── PartialOrd / Comparison ──────────────────────────────────────────
// Mirrors Rust PartialOrd for Value.
// Returns -1, 0, 1, or null (incomparable).

function compareUint8Arrays(a: Uint8Array, b: Uint8Array): number {
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    if (a[i] < b[i]) return -1;
    if (a[i] > b[i]) return 1;
  }
  if (a.length < b.length) return -1;
  if (a.length > b.length) return 1;
  return 0;
}

/** Partial comparison of two Values. Returns -1, 0, 1 for ordering, or null if incomparable. */
export function valuePartialCmp(a: Value, b: Value): number | null {
  if (a.type !== b.type) {
    return null; // Cross-type comparison: different types are not comparable
  }

  switch (a.type) {
    case 'I16': return a.value < (b as typeof a).value ? -1 : a.value > (b as typeof a).value ? 1 : 0;
    case 'I32': return a.value < (b as typeof a).value ? -1 : a.value > (b as typeof a).value ? 1 : 0;
    case 'I64': return a.value < (b as typeof a).value ? -1 : a.value > (b as typeof a).value ? 1 : 0;
    case 'F64': {
      const fa = a.value;
      const fb = (b as typeof a).value;
      if (isNaN(fa) || isNaN(fb)) return null;
      return fa < fb ? -1 : fa > fb ? 1 : 0;
    }
    case 'Bool': {
      const ba = a.value ? 1 : 0;
      const bb = (b as typeof a).value ? 1 : 0;
      return ba < bb ? -1 : ba > bb ? 1 : 0;
    }
    case 'String': {
      const sa = a.value;
      const sb = (b as typeof a).value;
      return sa < sb ? -1 : sa > sb ? 1 : 0;
    }
    case 'EntityId':
      return compareUint8Arrays(a.value.toBytes(), (b as typeof a).value.toBytes());
    case 'Object':
      return compareUint8Arrays(a.value, (b as typeof a).value);
    case 'Binary':
      return compareUint8Arrays(a.value, (b as typeof a).value);
    case 'Json': {
      // JSON values: compare by serialized form (not ideal but works for basic cases)
      const ja = JSON.stringify(a.value);
      const jb = JSON.stringify((b as typeof a).value);
      return ja < jb ? -1 : ja > jb ? 1 : 0;
    }
  }
}

// Comparison operators for Value (used in filter.rs). Mirrors Rust impl Value { gt, ge, lt, le }.

/** Returns true if a > b. */
export function valueGt(a: Value, b: Value): boolean {
  return valuePartialCmp(a, b) === 1;
}

/** Returns true if a >= b. */
export function valueGe(a: Value, b: Value): boolean {
  const cmp = valuePartialCmp(a, b);
  return cmp === 1 || cmp === 0;
}

/** Returns true if a < b. */
export function valueLt(a: Value, b: Value): boolean {
  return valuePartialCmp(a, b) === -1;
}

/** Returns true if a <= b. */
export function valueLe(a: Value, b: Value): boolean {
  const cmp = valuePartialCmp(a, b);
  return cmp === -1 || cmp === 0;
}

// ── Value equality ───────────────────────────────────────────────────

/** Deep equality for Value. Mirrors Rust PartialEq for Value. */
export function valueEquals(a: Value, b: Value): boolean {
  if (a.type !== b.type) return false;
  switch (a.type) {
    case 'I16':
    case 'I32':
    case 'I64':
    case 'F64':
    case 'Bool':
    case 'String':
      return a.value === (b as typeof a).value;
    case 'EntityId':
      return a.value.equals((b as { type: 'EntityId'; value: EntityId }).value);
    case 'Object':
    case 'Binary':
      return compareUint8Arrays(a.value, (b as typeof a).value) === 0;
    case 'Json':
      return JSON.stringify(a.value) === JSON.stringify((b as typeof a).value);
  }
}

// ── Display ──────────────────────────────────────────────────────────
// Mirrors Rust Display for Value.

export function valueToString(v: Value): string {
  switch (v.type) {
    case 'I16': return String(v.value);
    case 'I32': return String(v.value);
    case 'I64': return String(v.value);
    case 'F64': return String(v.value);
    case 'Bool': return String(v.value);
    case 'String': return JSON.stringify(v.value);
    case 'EntityId': return v.value.toString();
    case 'Object': return `[Object bytes(${v.value.length})]`;
    case 'Binary': return `[Binary bytes(${v.value.length})]`;
    case 'Json': return JSON.stringify(v.value);
  }
}

// ── extract_at_path ──────────────────────────────────────────────────
// Mirrors Rust Value::extract_at_path().

/** Convert a plain JSON value to a Value union member. Mirrors Rust json_value_to_value(). */
function jsonValueToValue(json: unknown): Value {
  if (json === null || json === undefined) {
    return { type: 'Json', value: null };
  }
  if (typeof json === 'boolean') {
    return { type: 'Bool', value: json };
  }
  if (typeof json === 'number') {
    if (Number.isInteger(json)) {
      return { type: 'I64', value: json };
    }
    return { type: 'F64', value: json };
  }
  if (typeof json === 'string') {
    return { type: 'String', value: json };
  }
  // Arrays and objects remain as Json
  return { type: 'Json', value: json };
}

/**
 * Extract value at a sub-path within structured data.
 * Returns null if the path doesn't exist (missing -- distinct from null).
 * For empty path, returns the value unchanged.
 * Supports Json, Binary, and String (permissive for backward compat).
 * Mirrors Rust Value::extract_at_path().
 */
export function extractAtPath(value: Value, path: string[]): Value | null {
  if (path.length === 0) {
    return value;
  }

  switch (value.type) {
    case 'Json': {
      let current: unknown = value.value;
      for (const key of path) {
        if (current === null || current === undefined || typeof current !== 'object') return null;
        current = (current as Record<string, unknown>)[key];
        if (current === undefined) return null;
      }
      return jsonValueToValue(current);
    }
    case 'Binary': {
      // Attempt to parse binary as JSON
      let json: unknown;
      try {
        const text = new TextDecoder().decode(value.value);
        json = JSON.parse(text);
      } catch {
        return null;
      }
      let current: unknown = json;
      for (const key of path) {
        if (current === null || current === undefined || typeof current !== 'object') return null;
        current = (current as Record<string, unknown>)[key];
        if (current === undefined) return null;
      }
      return jsonValueToValue(current);
    }
    case 'String': {
      // Attempt to parse string as JSON
      let json: unknown;
      try {
        json = JSON.parse(value.value);
      } catch {
        return null;
      }
      let current: unknown = json;
      for (const key of path) {
        if (current === null || current === undefined || typeof current !== 'object') return null;
        current = (current as Record<string, unknown>)[key];
        if (current === undefined) return null;
      }
      return jsonValueToValue(current);
    }
    default:
      return null;
  }
}

// ── Literal <-> Value conversions ────────────────────────────────────
// Mirrors Rust From<Literal> for Value and From<Value> for Literal.

/** Convert an AnkQL Literal to a Value. Mirrors Rust From<ankql::ast::Literal> for Value. */
export function valueFromLiteral(literal: Literal): Value {
  switch (literal.type) {
    case 'I16': return { type: 'I16', value: literal.value };
    case 'I32': return { type: 'I32', value: literal.value };
    case 'I64': return { type: 'I64', value: Number(literal.value) };
    case 'F64': return { type: 'F64', value: literal.value };
    case 'Bool': return { type: 'Bool', value: literal.value };
    case 'String': return { type: 'String', value: literal.value };
    case 'EntityId': return { type: 'EntityId', value: EntityId.fromBytes(literal.value) };
    case 'Object': return { type: 'Object', value: literal.value };
    case 'Binary': return { type: 'Binary', value: literal.value };
    case 'Json': return { type: 'Json', value: literal.value };
  }
}

/** Convert a Value to an AnkQL Literal. Mirrors Rust From<Value> for ankql::ast::Literal. */
export function valueToLiteral(value: Value): Literal {
  switch (value.type) {
    case 'I16': return { type: 'I16', value: value.value };
    case 'I32': return { type: 'I32', value: value.value };
    case 'I64': return { type: 'I64', value: BigInt(value.value) };
    case 'F64': return { type: 'F64', value: value.value };
    case 'Bool': return { type: 'Bool', value: value.value };
    case 'String': return { type: 'String', value: value.value };
    case 'EntityId': return { type: 'EntityId', value: value.value.toBytes() };
    case 'Object': {
      // Mirrors Rust: Object bytes -> String via lossy UTF-8
      const text = new TextDecoder('utf-8', { fatal: false }).decode(value.value);
      return { type: 'String', value: text };
    }
    case 'Binary': {
      // Mirrors Rust: Binary bytes -> String via lossy UTF-8
      const text = new TextDecoder('utf-8', { fatal: false }).decode(value.value);
      return { type: 'String', value: text };
    }
    case 'Json': return { type: 'Json', value: value.value };
  }
}
