// MIRRORS: ankurah/core/src/selection/filter.rs

import { type Predicate, type Expr, type ComparisonOperator, type PathExpr, type Literal } from '@ankurah/ankql';
import type { Value } from '../value/index.ts';
import { valueFromLiteral, valueEquals, valueGt, valueLt, valueGe, valueLe, extractAtPath } from '../value/index.ts';
import { valueType } from '../value/index.ts';
import { tryCastTo } from '../value/cast.ts';

// ---------------------------------------------------------------------------
// FilterError
// ---------------------------------------------------------------------------

/**
 * Error type for filter evaluation.
 *
 * Rust: `pub enum Error { CollectionMismatch, PropertyNotFound, UnsupportedExpression, UnsupportedOperator }`
 * TS: Error subclass with `kind` discriminant [A8].
 */
export type FilterErrorKind =
  | 'CollectionMismatch'
  | 'PropertyNotFound'
  | 'UnsupportedExpression'
  | 'UnsupportedOperator';

export class FilterError extends Error {
  readonly kind: FilterErrorKind;

  constructor(kind: FilterErrorKind, message: string) {
    super(message);
    this.name = 'FilterError';
    this.kind = kind;
  }

  static collectionMismatch(expected: string, actual: string): FilterError {
    return new FilterError('CollectionMismatch', `collection mismatch: expected ${expected}, got ${actual}`);
  }

  static propertyNotFound(name: string): FilterError {
    return new FilterError('PropertyNotFound', `property not found: ${name}`);
  }

  static unsupportedExpression(msg: string): FilterError {
    return new FilterError('UnsupportedExpression', `Unsupported expression: ${msg}`);
  }

  static unsupportedOperator(msg: string): FilterError {
    return new FilterError('UnsupportedOperator', `Unsupported operator: ${msg}`);
  }
}

// ---------------------------------------------------------------------------
// ExprOutput
// ---------------------------------------------------------------------------

/**
 * Internal discriminated union mirroring Rust `ExprOutput<Value>`.
 *
 * Rust: `pub enum ExprOutput<T> { List(Vec<ExprOutput<T>>), Value(T), None }`
 */
type ExprOutput =
  | { type: 'Value'; value: Value }
  | { type: 'List'; items: ExprOutput[] }
  | { type: 'None' };

/** Rust: `fn as_value(&self) -> Option<&T>` */
function exprOutputAsValue(output: ExprOutput): Value | null {
  if (output.type === 'Value') return output.value;
  return null;
}

/** Rust: `fn as_list(&self) -> Option<&Vec<ExprOutput<T>>>` */
function exprOutputAsList(output: ExprOutput): ExprOutput[] | null {
  if (output.type === 'List') return output.items;
  return null;
}

/** Rust: `fn is_none(&self) -> bool` */
function exprOutputIsNone(output: ExprOutput): boolean {
  return output.type === 'None';
}

// ---------------------------------------------------------------------------
// Filterable
// ---------------------------------------------------------------------------

/**
 * Trait for items that can be filtered by predicate evaluation.
 * Returns typed Values to enable proper comparison with casting.
 *
 * Rust: `pub trait Filterable { fn collection(&self) -> &str; fn value(&self, name: &str) -> Option<Value>; }`
 */
export interface Filterable {
  collection(): string;
  value(name: string): Value | null;
}

// ---------------------------------------------------------------------------
// evaluate_expr
// ---------------------------------------------------------------------------

/** Rust: `fn evaluate_expr<I: Filterable>(item: &I, expr: &Expr) -> Result<ExprOutput<Value>, Error>` */
function evaluateExpr(item: Filterable, expr: Expr): ExprOutput | FilterError {
  return expr.match({
    Placeholder: () =>
      FilterError.propertyNotFound('Placeholder values must be replaced before filtering'),

    Literal: (v) =>
      ({ type: 'Value', value: valueFromLiteral(v.literal) }) as ExprOutput,

    Path: (v) => {
      const path = v.path;

      // For simple paths, use the first step as the property name
      if (path.isSimple()) {
        const name = path.first();
        const val = item.value(name);
        if (val === null) return FilterError.propertyNotFound(name);
        return { type: 'Value', value: val } as ExprOutput;
      }

      // Multi-step path - could be:
      // 1. Collection.property (legacy, check if first step matches collection)
      // 2. property.nested.path (JSON traversal)
      const first = path.first();

      // First, check if it's a collection-qualified path
      if (first === item.collection()) {
        // Treat remaining path as property access
        const remaining = path.steps.slice(1);
        if (remaining.length === 1) {
          // Simple collection.property
          const name = remaining[0];
          const val = item.value(name);
          if (val === null) return FilterError.propertyNotFound(name);
          return { type: 'Value', value: val } as ExprOutput;
        }
        // collection.property.nested... - get property and traverse sub-path
        const propertyName = remaining[0];
        const subPath = remaining.slice(1);
        return evaluateSubPath(item, propertyName, subPath);
      }

      // Not a collection qualifier - treat first step as property, rest as sub-path
      const propertyName = first;
      const subPath = path.steps.slice(1);
      return evaluateSubPath(item, propertyName, subPath);
    },

    ExprList: (v) => {
      const results: ExprOutput[] = [];
      for (const e of v.exprs) {
        const result = evaluateExpr(item, e);
        if (result instanceof FilterError) return result;
        results.push(result);
      }
      return { type: 'List', items: results } as ExprOutput;
    },

    Predicate: () =>
      FilterError.unsupportedExpression('Only literal, path, and list expressions are supported'),
    InfixExpr: () =>
      FilterError.unsupportedExpression('Only literal, path, and list expressions are supported'),
  });
}

// ---------------------------------------------------------------------------
// evaluate_sub_path
// ---------------------------------------------------------------------------

/**
 * Evaluate a sub-path traversal: get property value, extract nested value at path.
 * Delegates to extractAtPath for the actual traversal.
 *
 * Rust: `fn evaluate_sub_path<I: Filterable>(item: &I, property_name: &str, sub_path: &[impl AsRef<str>]) -> Result<ExprOutput<Value>, Error>`
 */
function evaluateSubPath(item: Filterable, propertyName: string, subPath: string[]): ExprOutput | FilterError {
  const propertyValue = item.value(propertyName);
  if (propertyValue === null) {
    return FilterError.propertyNotFound(propertyName);
  }

  const extracted = extractAtPath(propertyValue, subPath);
  if (extracted === null) {
    return FilterError.propertyNotFound(
      `Sub-path '${subPath.join('.')}' not found in property '${propertyName}'`,
    );
  }
  return { type: 'Value', value: extracted };
}

// ---------------------------------------------------------------------------
// compare_values_with_cast
// ---------------------------------------------------------------------------

/**
 * Compare two values with automatic casting (for regular schema-typed fields).
 * If types don't match, attempts to cast right to left's type, then left to right's type.
 *
 * Rust: `fn compare_values_with_cast(left: &Value, right: &Value, op: impl Fn(&Value, &Value) -> bool) -> bool`
 */
function compareValuesWithCast(left: Value, right: Value, op: (a: Value, b: Value) => boolean): boolean {
  // If types match, compare directly
  if (valueType(left) === valueType(right)) {
    return op(left, right);
  }

  // Try casting right to left's type
  const castedRight = tryCastTo(right, valueType(left));
  if (castedRight !== null) {
    return op(left, castedRight);
  }

  // Try casting left to right's type
  const castedLeft = tryCastTo(left, valueType(right));
  if (castedLeft !== null) {
    return op(castedLeft, right);
  }

  // No valid cast, types incompatible
  return false;
}

// ---------------------------------------------------------------------------
// evaluate_predicate
// ---------------------------------------------------------------------------

/**
 * Main function: recursively evaluates an AnkQL predicate against a filterable item.
 *
 * Rust: `pub fn evaluate_predicate<I: Filterable>(item: &I, predicate: &Predicate) -> Result<bool, Error>`
 * Divergence: Returns Result-like [boolean, FilterError | null] instead of Result<bool, Error>.
 * Use evaluatePredicateChecked for error propagation, or evaluatePredicate for simple boolean.
 */
export function evaluatePredicateChecked(item: Filterable, predicate: Predicate): [boolean, FilterError | null] {
  return predicate.match({
    True: () => [true, null] as [boolean, FilterError | null],

    False: () => [false, null] as [boolean, FilterError | null],

    Placeholder: () =>
      [false, FilterError.propertyNotFound('Placeholder must be transformed before filtering')] as [boolean, FilterError | null],

    And: (v) => {
      const [left, leftErr] = evaluatePredicateChecked(item, v.left);
      if (leftErr) return [false, leftErr];
      if (!left) return [false, null];
      return evaluatePredicateChecked(item, v.right);
    },

    Or: (v) => {
      const [left, leftErr] = evaluatePredicateChecked(item, v.left);
      if (leftErr) return [false, leftErr];
      if (left) return [true, null];
      return evaluatePredicateChecked(item, v.right);
    },

    Not: (v) => {
      const [result, err] = evaluatePredicateChecked(item, v.predicate);
      if (err) return [false, err];
      return [!result, null];
    },

    IsNull: (v) => {
      const result = evaluateExpr(item, v.expr);
      if (result instanceof FilterError) return [false, result];
      return [exprOutputIsNone(result), null];
    },

    Comparison: (v) => {
      const leftVal = evaluateExpr(item, v.left);
      if (leftVal instanceof FilterError) return [false, leftVal];
      const rightVal = evaluateExpr(item, v.right);
      if (rightVal instanceof FilterError) return [false, rightVal];

      return v.operator.match({
        Equal: () => {
          const l = exprOutputAsValue(leftVal);
          const r = exprOutputAsValue(rightVal);
          if (l === null || r === null) return [false, null] as [boolean, FilterError | null];
          return [compareValuesWithCast(l, r, (a, b) => valueEquals(a, b)), null];
        },
        NotEqual: () => {
          const l = exprOutputAsValue(leftVal);
          const r = exprOutputAsValue(rightVal);
          if (l === null || r === null) return [false, null] as [boolean, FilterError | null];
          return [compareValuesWithCast(l, r, (a, b) => !valueEquals(a, b)), null];
        },
        GreaterThan: () => {
          const l = exprOutputAsValue(leftVal);
          const r = exprOutputAsValue(rightVal);
          if (l === null || r === null) return [false, null] as [boolean, FilterError | null];
          return [compareValuesWithCast(l, r, (a, b) => valueGt(a, b)), null];
        },
        GreaterThanOrEqual: () => {
          const l = exprOutputAsValue(leftVal);
          const r = exprOutputAsValue(rightVal);
          if (l === null || r === null) return [false, null] as [boolean, FilterError | null];
          return [compareValuesWithCast(l, r, (a, b) => valueGe(a, b)), null];
        },
        LessThan: () => {
          const l = exprOutputAsValue(leftVal);
          const r = exprOutputAsValue(rightVal);
          if (l === null || r === null) return [false, null] as [boolean, FilterError | null];
          return [compareValuesWithCast(l, r, (a, b) => valueLt(a, b)), null];
        },
        LessThanOrEqual: () => {
          const l = exprOutputAsValue(leftVal);
          const r = exprOutputAsValue(rightVal);
          if (l === null || r === null) return [false, null] as [boolean, FilterError | null];
          return [compareValuesWithCast(l, r, (a, b) => valueLe(a, b)), null];
        },
        In: () => {
          const value = exprOutputAsValue(leftVal);
          const list = exprOutputAsList(rightVal);
          if (value === null || list === null) {
            if (value === null) return [false, FilterError.propertyNotFound('Expected single value for IN left operand')] as [boolean, FilterError | null];
            return [false, FilterError.propertyNotFound('Expected list for IN right operand')] as [boolean, FilterError | null];
          }
          return [list.some((listItem) => {
            const lv = exprOutputAsValue(listItem);
            if (lv === null) return false;
            return compareValuesWithCast(value, lv, (a, b) => valueEquals(a, b));
          }), null] as [boolean, FilterError | null];
        },
        Between: () =>
          [false, FilterError.unsupportedOperator('BETWEEN operator not yet supported')] as [boolean, FilterError | null],
      });
    },
  });
}

/**
 * Convenience wrapper: evaluates predicate, returns boolean (errors become false).
 * Backward-compatible with existing call sites.
 */
export function evaluatePredicate(item: Filterable, predicate: Predicate): boolean {
  const [result] = evaluatePredicateChecked(item, predicate);
  return result;
}

// ---------------------------------------------------------------------------
// FilterResult
// ---------------------------------------------------------------------------

/**
 * Result type for filter iteration.
 *
 * Rust: `pub enum FilterResult<R> { Pass(R), Skip(R), Error(R, Error) }`
 */
export type FilterResult<R> =
  | { type: 'Pass'; item: R }
  | { type: 'Skip'; item: R }
  | { type: 'Error'; item: R; error: FilterError };

// ---------------------------------------------------------------------------
// FilterIterator
// ---------------------------------------------------------------------------

/**
 * Iterator that filters items based on a predicate, yielding FilterResult.
 *
 * Rust: `pub struct FilterIterator<I> { iter: I, predicate: Predicate }`
 * Divergence: TS uses generator function yielding FilterResult [E8].
 */
export function* filterIterator<R extends Filterable>(
  items: Iterable<R>,
  predicate: Predicate,
): Generator<FilterResult<R>> {
  for (const item of items) {
    const [result, error] = evaluatePredicateChecked(item, predicate);
    if (error !== null) {
      yield { type: 'Error', item, error };
    } else if (result) {
      yield { type: 'Pass', item };
    } else {
      yield { type: 'Skip', item };
    }
  }
}
