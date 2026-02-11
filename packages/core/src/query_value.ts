// MIRRORS: ankurah/core/src/query_value.rs

import type { EntityId } from '@ankurah/proto';
import { EntityId as EntityIdClass } from '@ankurah/proto';
import type { Expr, Literal } from '@ankurah/ankql';
import { ParseError } from '@ankurah/ankql';

// ---------------------------------------------------------------------------
// QueryValue — value type for query parameter substitution
// ---------------------------------------------------------------------------

/**
 * Value type for query parameter substitution.
 *
 * Rust: `pub enum QueryValue { String, Int, Float, Bool, EntityId }`
 * TS: Discriminated union [A8].
 *
 * Used with `fetch()`, `query()`, etc. to fill in `?` placeholders:
 * ```
 * ops.fetch(ctx, "name = ?", [queryValueString("Alice")])
 * ```
 */
export type QueryValue =
  | { type: 'String'; value: string }
  | { type: 'Int'; value: number }
  | { type: 'Float'; value: number }
  | { type: 'Bool'; value: boolean }
  | { type: 'EntityId'; value: string }; // base64 string for compatibility, matching Rust FFI

// ---------------------------------------------------------------------------
// Factory helpers (matching Rust From impls)
// ---------------------------------------------------------------------------

/** Create a String QueryValue. Mirrors Rust `From<String> for QueryValue`. */
export function queryValueString(s: string): QueryValue {
  return { type: 'String', value: s };
}

/** Create an Int QueryValue. Mirrors Rust `From<i64> for QueryValue`. */
export function queryValueInt(i: number): QueryValue {
  return { type: 'Int', value: i };
}

/** Create a Float QueryValue. Mirrors Rust `From<f64> for QueryValue`. */
export function queryValueFloat(f: number): QueryValue {
  return { type: 'Float', value: f };
}

/** Create a Bool QueryValue. Mirrors Rust `From<bool> for QueryValue`. */
export function queryValueBool(b: boolean): QueryValue {
  return { type: 'Bool', value: b };
}

/** Create an EntityId QueryValue. Mirrors Rust `From<EntityId> for QueryValue`. */
export function queryValueEntityId(id: EntityId): QueryValue {
  return { type: 'EntityId', value: id.toBase64() };
}

// ---------------------------------------------------------------------------
// Conversion: QueryValue -> Expr
// ---------------------------------------------------------------------------

/**
 * Convert a QueryValue to an AnkQL Expr.
 *
 * Rust: `impl TryFrom<QueryValue> for ankql::ast::Expr`
 * Throws ParseError on invalid EntityId.
 */
export function queryValueToExpr(qv: QueryValue): Expr {
  switch (qv.type) {
    case 'String':
      return { type: 'Literal', value: { type: 'String', value: qv.value } satisfies Literal };
    case 'Int':
      // Divergence: Rust uses i64 -> Literal::I64(bigint). TS uses number -> I64(bigint) [E3].
      return { type: 'Literal', value: { type: 'I64', value: BigInt(qv.value) } satisfies Literal };
    case 'Float':
      return { type: 'Literal', value: { type: 'F64', value: qv.value } satisfies Literal };
    case 'Bool':
      return { type: 'Literal', value: { type: 'Bool', value: qv.value } satisfies Literal };
    case 'EntityId': {
      const id = EntityIdClass.fromBase64(qv.value);
      if (!id) {
        throw new ParseError(`Invalid EntityId: ${qv.value}`);
      }
      return { type: 'Literal', value: { type: 'EntityId', value: id.toBytes() } satisfies Literal };
    }
  }
}
