// MIRRORS: ankurah/core/src/value/cast_predicate.rs

import type { Expr, Literal, Predicate } from '@ankurah/ankql';
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
  switch (predicate.type) {
    case 'Comparison': {
      const { left, operator, right } = predicate;

      // Handle both cases: field = literal AND literal = field
      if (left.type === 'Path' && right.type === 'Literal') {
        // Case 1: field = literal (cast literal to field type)
        const targetType = schema.fieldType(left.value);
        const castLiteral = castLiteralToType(right.value, targetType);
        return { type: 'Comparison', left, operator, right: castLiteral };
      }
      if (left.type === 'Literal' && right.type === 'Path') {
        // Case 2: literal = field (cast literal to field type)
        const targetType = schema.fieldType(right.value);
        const castLiteral = castLiteralToType(left.value, targetType);
        return { type: 'Comparison', left: castLiteral, operator, right };
      }

      // For all other cases, recursively cast both sides
      const castLeft = castExprTypes(left, schema);
      const castRight = castExprTypes(right, schema);
      return { type: 'Comparison', left: castLeft, operator, right: castRight };
    }
    case 'IsNull':
      return { type: 'IsNull', expr: castExprTypes(predicate.expr, schema) };
    case 'And':
      return {
        type: 'And',
        left: castPredicateTypes(predicate.left, schema),
        right: castPredicateTypes(predicate.right, schema),
      };
    case 'Or':
      return {
        type: 'Or',
        left: castPredicateTypes(predicate.left, schema),
        right: castPredicateTypes(predicate.right, schema),
      };
    case 'Not':
      return { type: 'Not', predicate: castPredicateTypes(predicate.predicate, schema) };
    case 'True':
    case 'False':
    case 'Placeholder':
      return predicate;
  }
}

// ── castExprTypes (private) ──────────────────────────────────────────

/** Cast all literals in an expression based on field names. Mirrors Rust cast_expr_types(). */
function castExprTypes(expr: Expr, schema: CollectionSchema): Expr {
  switch (expr.type) {
    case 'Literal':
      return expr; // Literals are cast in context
    case 'Path':
      return expr;
    case 'Predicate':
      return { type: 'Predicate', value: castPredicateTypes(expr.value, schema) };
    case 'InfixExpr':
      return {
        type: 'InfixExpr',
        left: castExprTypes(expr.left, schema),
        operator: expr.operator,
        right: castExprTypes(expr.right, schema),
      };
    case 'ExprList':
      return {
        type: 'ExprList',
        values: expr.values.map((e) => castExprTypes(e, schema)),
      };
    case 'Placeholder':
      return expr;
  }
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
    return { type: 'Literal', value: castLiteral };
  } catch (e) {
    if (e instanceof CastErrorException) {
      throw RetrievalError.storageError(new Error(`Type casting error: ${e.message}`));
    }
    throw e;
  }
}
