// MIRRORS: ankurah/core/src/value/cast_predicate.rs

import { Expr, Literal, Predicate, PathExpr, ComparisonOperator } from '@ankurah/ankql';
import type { Value } from './index';
import { ValueType, valueFromLiteral, valueToLiteral } from './index';
import { castTo, CastErrorException } from './cast';
import { RetrievalError } from '../error.ts';
export type { CollectionSchema } from '../schema.ts';
import type { CollectionSchema } from '../schema.ts';

// ── castPredicateTypes ───────────────────────────────────────────────

/** Cast all literals in a predicate based on field names using a CollectionSchema.
 *  Mirrors Rust cast_predicate_types(). */
export function castPredicateTypes(predicate: Predicate, schema: CollectionSchema): Predicate {
  return predicate.match({
    Comparison: (v) => {
      const { left, operator, right } = v;

      // Handle both cases: field = literal AND literal = field
      if (left.is('Path') && right.is('Literal')) {
        // Case 1: field = literal (cast literal to field type)
        const path = (left.value as { path: PathExpr }).path;
        const literal = (right.value as { literal: Literal }).literal;
        const targetType = schema.fieldType(path);
        const castLit = castLiteralToType(literal, targetType);
        return Predicate.Comparison(left, operator, castLit);
      }
      if (left.is('Literal') && right.is('Path')) {
        // Case 2: literal = field (cast literal to field type)
        const literal = (left.value as { literal: Literal }).literal;
        const path = (right.value as { path: PathExpr }).path;
        const targetType = schema.fieldType(path);
        const castLit = castLiteralToType(literal, targetType);
        return Predicate.Comparison(castLit, operator, right);
      }

      // For all other cases, recursively cast both sides
      const castLeft = castExprTypes(left, schema);
      const castRight = castExprTypes(right, schema);
      return Predicate.Comparison(castLeft, operator, castRight);
    },
    IsNull: (v) => Predicate.IsNull(castExprTypes(v.expr, schema)),
    And: (v) => Predicate.And(
      castPredicateTypes(v.left, schema),
      castPredicateTypes(v.right, schema),
    ),
    Or: (v) => Predicate.Or(
      castPredicateTypes(v.left, schema),
      castPredicateTypes(v.right, schema),
    ),
    Not: (v) => Predicate.Not(castPredicateTypes(v.predicate, schema)),
    True: () => predicate,
    False: () => predicate,
    Placeholder: () => predicate,
  });
}

// ── castExprTypes (private) ──────────────────────────────────────────

/** Cast all literals in an expression based on field names. Mirrors Rust cast_expr_types(). */
function castExprTypes(expr: Expr, schema: CollectionSchema): Expr {
  return expr.match({
    Literal: () => expr, // Literals are cast in context
    Path: () => expr,
    Predicate: (v) => Expr.Predicate(castPredicateTypes(v.predicate, schema)),
    InfixExpr: (v) => Expr.InfixExpr(
      castExprTypes(v.left, schema),
      v.operator,
      castExprTypes(v.right, schema),
    ),
    ExprList: (v) => Expr.ExprList(v.exprs.map((e) => castExprTypes(e, schema))),
    Placeholder: () => expr,
  });
}

// ── castLiteralToType (private) ──────────────────────────────────────

/** Cast a literal to a specific type using the Value casting system.
 *  Mirrors Rust cast_literal_to_type(). */
function castLiteralToType(literal: Literal, targetType: ValueType): Expr {
  // Convert Literal -> Value -> cast -> Literal -> Expr
  const value: Value = valueFromLiteral(literal);
  try {
    const castValue = castTo(value, targetType);
    const castLiteral = valueToLiteral(castValue);
    return Expr.Literal(castLiteral);
  } catch (e) {
    if (e instanceof CastErrorException) {
      throw RetrievalError.storageError(new Error(`Type casting error: ${e.message}`));
    }
    throw e;
  }
}
