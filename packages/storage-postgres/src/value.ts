// MIRRORS: ankurah/storage/postgres/src/value.rs

import type { Value } from '@ankurah/core';
import type { EntityId } from '@ankurah/proto';

// ── PGValue ──────────────────────────────────────────────────────────
// Represents a typed PostgreSQL value for parameterized queries.

export type PGValue =
  | { type: 'Bytea'; value: Uint8Array }
  | { type: 'CharacterVarying'; value: string }
  | { type: 'SmallInt'; value: number }
  | { type: 'Integer'; value: number }
  | { type: 'BigInt'; value: bigint }
  | { type: 'DoublePrecision'; value: number }
  | { type: 'Boolean'; value: boolean }
  /// JSON value - stored as PostgreSQL's native jsonb type for query support.
  | { type: 'Jsonb'; value: unknown };

// impl PGValue

export function pgValuePostgresType(pgValue: PGValue): string {
  switch (pgValue.type) {
    case 'CharacterVarying': return 'varchar';
    case 'SmallInt': return 'int2';
    case 'Integer': return 'int4';
    case 'BigInt': return 'int8';
    case 'DoublePrecision': return 'float8';
    case 'Bytea': return 'bytea';
    case 'Boolean': return 'boolean';
    case 'Jsonb': return 'jsonb';
  }
}

// impl From<Value> for PGValue

export function pgValueFromValue(value: Value): PGValue {
  switch (value.type) {
    case 'String': return { type: 'CharacterVarying', value: value.value };
    case 'I16': return { type: 'SmallInt', value: value.value };
    case 'I32': return { type: 'Integer', value: value.value };
    case 'I64': return { type: 'BigInt', value: BigInt(value.value) };
    case 'F64': return { type: 'DoublePrecision', value: value.value };
    case 'Bool': return { type: 'Boolean', value: value.value };
    case 'EntityId': return { type: 'CharacterVarying', value: (value.value as EntityId).toBase64() };
    case 'Object': return { type: 'Bytea', value: value.value as Uint8Array };
    case 'Binary': return { type: 'Bytea', value: value.value as Uint8Array };
    case 'Json': return { type: 'Jsonb', value: value.value };
  }
}
