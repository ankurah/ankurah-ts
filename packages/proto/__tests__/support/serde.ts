// TS-ONLY: render a decoded TypeScript value in the shape `serde_json` gives the
// same Rust value, so a fixture sidecar can be compared against it directly.
//
// This exists to catch the bug byte-equality cannot see: a decoder that swaps two
// adjacent same-typed fields round-trips its own output perfectly and matches the
// fixture bytes, while handing the application the wrong values. The sidecar
// records what each value must decode to; this module puts the decoded object into
// the same shape so `expect(...).toEqual(...)` is a real comparison.
//
// The rules it applies are serde's, not this port's:
//   - a unit enum variant is the bare variant name, a newtype variant is
//     `{Variant: inner}`, a tuple variant is `{Variant: [a, b]}`, a struct variant
//     is `{Variant: {field: …}}`
//   - a newtype struct is its inner value with no wrapper
//   - `Vec<u8>` is an array of numbers; `[u8; N]` behind a hand-written
//     `Serialize` (EntityId, EventId) is an unpadded base64url string
//   - ULID-backed ids are their 26-character Crockford Base32 rendering
//   - struct fields are the Rust names, so camelCase comes back to snake_case
//
// Two tolerances, both deliberate. A `bigint` and a `number` of the same integer
// value compare equal, because whether a port holds a small `i64` as one or the
// other is a representation choice, not a wire bug — a value the port has actually
// rounded (`9007199254740993` decoded through a JS number) still differs and still
// fails. And `Literal::Json`, whose Rust form is the UTF-8 bytes of the serialized
// document, is re-serialized here with `JSON.stringify`, which matches
// `serde_json::to_vec` for the documents these fixtures carry.

import { Enum } from '@ankurah/base';

const textEncoder = new TextEncoder();

/** camelCase field name back to the Rust declaration name. */
export function snakeCase(name: string): string {
  return name.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase();
}

/** Unpadded base64url, the form EntityId/EventId's hand-written Serialize writes. */
function base64url(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

/** 26-character Crockford Base32, the form `ulid::Ulid`'s Serialize writes. */
function ulidString(bytes: Uint8Array): string {
  let value = 0n;
  for (const b of bytes) value = (value << 8n) | BigInt(b);
  let out = '';
  for (let i = 0; i < 26; i++) {
    out = CROCKFORD[Number(value & 31n)] + out;
    value >>= 5n;
  }
  return out;
}

// ── ankql AST shapes ────────────────────────────────────────────────────────
//
// The ankql AST enums carry named fields in TypeScript (`{value: …}`,
// `{literal: …}`) where Rust has positional ones, so the tuple/newtype shape
// cannot be read off the object. These tables say which Rust shape each variant
// really has. Every other enum in the port names its positional fields `_0`,
// `_1`, … and needs no entry.

const NEWTYPE_VARIANTS: Record<string, Record<string, string>> = {
  Literal: {
    I16: 'value', I32: 'value', I64: 'value', F64: 'value', Bool: 'value',
    String: 'value', EntityId: 'value', Object: 'value', Binary: 'value', Json: 'value',
  },
  Expr: { Literal: 'literal', Path: 'path', Predicate: 'predicate', ExprList: 'exprs' },
  Predicate: { IsNull: 'expr', Not: 'predicate' },
};

const TUPLE_VARIANTS: Record<string, Record<string, string[]>> = {
  Predicate: { And: ['left', 'right'], Or: ['left', 'right'] },
};

/** Variants whose payload serializes as something other than the field's own JS shape. */
function literalPayload(variant: string, value: unknown): unknown {
  switch (variant) {
    case 'EntityId':
      // Rust `Literal::EntityId(Ulid)` — a ULID string in both JSON and bincode.
      return value instanceof Uint8Array ? ulidString(value) : toSerde(value);
    case 'Json':
      // Rust `#[serde(with = "json_as_bytes")]` — the UTF-8 bytes of the document.
      return Array.from(textEncoder.encode(JSON.stringify(value)));
    default:
      return toSerde(value);
  }
}

// ── The converter ───────────────────────────────────────────────────────────

export function toSerde(v: unknown): unknown {
  if (v === null || v === undefined) return null;

  const t = typeof v;
  if (t === 'string' || t === 'boolean' || t === 'number') return v;
  if (t === 'bigint') {
    const b = v as bigint;
    // Fold into a number when a number holds it exactly, so a port that keeps a
    // small i64 as a bigint is not reported as a mismatch. Anything larger stays
    // a bigint and still differs from a rounded number.
    if (b >= BigInt(Number.MIN_SAFE_INTEGER) && b <= BigInt(Number.MAX_SAFE_INTEGER)) return Number(b);
    return b;
  }

  if (v instanceof Uint8Array) return Array.from(v);
  if (Array.isArray(v)) return v.map(toSerde);
  if (v instanceof Map) {
    const out: Record<string, unknown> = {};
    for (const [k, val] of v) out[String(k)] = toSerde(val);
    return out;
  }

  const obj = v as Record<string, unknown> & { constructor: { name: string } };
  const className = obj.constructor?.name ?? '';

  // Ids with a hand-written Serialize.
  if ((className === 'EntityId' || className === 'EventId') && obj.bytes instanceof Uint8Array) {
    return base64url(obj.bytes);
  }
  if (
    (className === 'TransactionId' || className === 'RequestId' || className === 'QueryId' || className === 'UpdateId') &&
    obj._0 instanceof Uint8Array
  ) {
    return ulidString(obj._0 as Uint8Array);
  }

  if (v instanceof Enum) {
    const variant = v.type as string;
    const payload = (v.value ?? {}) as Record<string, unknown>;
    const keys = Object.keys(payload);

    if (keys.length === 0) return variant;

    const newtypeField = NEWTYPE_VARIANTS[className]?.[variant];
    if (newtypeField !== undefined) {
      return { [variant]: className === 'Literal' ? literalPayload(variant, payload[newtypeField]) : toSerde(payload[newtypeField]) };
    }

    const tupleFields = TUPLE_VARIANTS[className]?.[variant];
    if (tupleFields !== undefined) {
      return { [variant]: tupleFields.map((f) => toSerde(payload[f])) };
    }

    // Positional payloads are named _0, _1, … by the transpiler.
    if (keys.every((k) => /^_\d+$/.test(k))) {
      const ordered = keys.slice().sort((a, b) => Number(a.slice(1)) - Number(b.slice(1)));
      if (ordered.length === 1) return { [variant]: toSerde(payload[ordered[0]]) };
      return { [variant]: ordered.map((k) => toSerde(payload[k])) };
    }

    const fields: Record<string, unknown> = {};
    for (const k of keys) fields[snakeCase(k)] = toSerde(payload[k]);
    return { [variant]: fields };
  }

  // Structs, including newtype structs, whose single field the transpiler names _0.
  const keys = Object.keys(obj);
  if (keys.length === 1 && keys[0] === '_0') return toSerde(obj._0);
  const fields: Record<string, unknown> = {};
  for (const k of keys) fields[snakeCase(k)] = toSerde(obj[k]);
  return fields;
}

// ── core::value::Value ──────────────────────────────────────────────────────

/**
 * `ankurah_core::value::Value` in serde's externally-tagged form: `{"I32": 5}`,
 * `{"String": "x"}`, and a cleared property as a bare `null`. Kept structural — it
 * reads the `{type, value}` union the port uses and never imports core, so proto,
 * core and storage tests can all share it.
 */
export function coreValueToSerde(v: unknown): unknown {
  if (v === null || v === undefined) return null;
  const value = v as { type: string; value: unknown };
  switch (value.type) {
    case 'EntityId': {
      // core's Value::EntityId holds a proto EntityId, whose Serialize is base64.
      const id = value.value as { bytes?: Uint8Array; toBytes?: () => Uint8Array };
      const bytes = id.bytes ?? id.toBytes?.();
      return { EntityId: bytes ? base64url(bytes) : toSerde(value.value) };
    }
    case 'Json':
      return { Json: Array.from(textEncoder.encode(JSON.stringify(value.value))) };
    default:
      return { [value.type]: toSerde(value.value) };
  }
}

/** A `BTreeMap<PropertyName, Option<Value>>` as the sidecars render it. */
export function propertyMapToSerde(map: Map<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of map) out[k] = coreValueToSerde(v);
  return out;
}
