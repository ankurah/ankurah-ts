// MIRRORS: ankurah/core/src/selection/filter.rs

import type { Predicate, Expr, ComparisonOperator, PathExpr, Literal } from '@ankurah/ankql';
import type { Value } from '../value/index.ts';
import { valueFromLiteral, valueEquals, valueGt, valueLt, valueGe, valueLe, extractAtPath, valuePartialCmp } from '../value/index.ts';
import { valueType, ValueType } from '../value/index.ts';
import { tryCastTo } from '../value/cast.ts';
import type { CollectionId } from '@ankurah/proto';

// ── ExprOutput ───────────────────────────────────────────────────────
// Internal discriminated union mirroring Rust ExprOutput<Value>.

type ExprOutput =
  | { type: 'Value'; value: Value }
  | { type: 'List'; items: ExprOutput[] }
  | { type: 'None' };

function exprOutputAsValue(output: ExprOutput): Value | null {
  if (output.type === 'Value') return output.value;
  return null;
}

function exprOutputAsList(output: ExprOutput): ExprOutput[] | null {
  if (output.type === 'List') return output.items;
  return null;
}

function exprOutputIsNone(output: ExprOutput): boolean {
  return output.type === 'None';
}

// ── Filterable ───────────────────────────────────────────────────────
// Trait for items that can be filtered by predicate evaluation.
// Returns typed Values to enable proper comparison with casting.

export interface Filterable {
  collection(): string;
  value(name: string): Value | null;
}

// ── compare_values_with_cast ─────────────────────────────────────────
// Compare two values with automatic casting (for regular schema-typed fields).
// If types don't match, attempts to cast right to left's type, then left to right's type.

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

// ── evaluate_sub_path ────────────────────────────────────────────────
// Evaluate a sub-path traversal: get property value, extract nested value at path.
// Delegates to extractAtPath for the actual traversal.

function evaluateSubPath(item: Filterable, propertyName: string, subPath: string[]): ExprOutput | null {
  const propertyValue = item.value(propertyName);
  if (propertyValue === null) {
    return null; // Property not found
  }

  const extracted = extractAtPath(propertyValue, subPath);
  if (extracted === null) {
    return null; // Sub-path not found
  }
  return { type: 'Value', value: extracted };
}

// ── evaluate_expr ────────────────────────────────────────────────────
// Evaluates an expression to produce a value. Returns null on error.

function evaluateExpr(item: Filterable, expr: Expr): ExprOutput | null {
  switch (expr.type) {
    case 'Placeholder':
      // Placeholder values must be replaced before filtering
      return null;

    case 'Literal':
      return { type: 'Value', value: valueFromLiteral(expr.value) };

    case 'Path': {
      const path: PathExpr = expr.value;

      // For simple paths, use the first step as the property name
      if (path.isSimple()) {
        const name = path.first();
        const val = item.value(name);
        if (val === null) return null;
        return { type: 'Value', value: val };
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
          if (val === null) return null;
          return { type: 'Value', value: val };
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
    }

    case 'ExprList': {
      const results: ExprOutput[] = [];
      for (const e of expr.values) {
        const result = evaluateExpr(item, e);
        if (result === null) return null;
        results.push(result);
      }
      return { type: 'List', items: results };
    }

    default:
      // InfixExpr, Predicate, etc. - unsupported for now
      return null;
  }
}

// ── evaluatePredicate ────────────────────────────────────────────────
// Main function: recursively evaluates an AnkQL predicate against a filterable item.
// Returns boolean (evaluation errors return false, matching the TS convention).

export function evaluatePredicate(item: Filterable, predicate: Predicate): boolean {
  switch (predicate.type) {
    case 'True':
      return true;

    case 'False':
      return false;

    case 'Placeholder':
      // Placeholder must be transformed before filtering
      return false;

    case 'And':
      return evaluatePredicate(item, predicate.left) && evaluatePredicate(item, predicate.right);

    case 'Or':
      return evaluatePredicate(item, predicate.left) || evaluatePredicate(item, predicate.right);

    case 'Not':
      return !evaluatePredicate(item, predicate.predicate);

    case 'IsNull': {
      const result = evaluateExpr(item, predicate.expr);
      return result === null || exprOutputIsNone(result);
    }

    case 'Comparison': {
      const leftVal = evaluateExpr(item, predicate.left);
      const rightVal = evaluateExpr(item, predicate.right);

      if (leftVal === null || rightVal === null) {
        return false;
      }

      return compareWithOperator(leftVal, rightVal, predicate.operator);
    }
  }
}

// ── compareWithOperator ──────────────────────────────────────────────
// Dispatches comparison based on the operator.

function compareWithOperator(leftVal: ExprOutput, rightVal: ExprOutput, operator: ComparisonOperator): boolean {
  switch (operator) {
    case 'Equal': {
      const l = exprOutputAsValue(leftVal);
      const r = exprOutputAsValue(rightVal);
      if (l === null || r === null) return false;
      return compareValuesWithCast(l, r, (a, b) => valueEquals(a, b));
    }

    case 'NotEqual': {
      const l = exprOutputAsValue(leftVal);
      const r = exprOutputAsValue(rightVal);
      if (l === null || r === null) return false;
      return compareValuesWithCast(l, r, (a, b) => !valueEquals(a, b));
    }

    case 'GreaterThan': {
      const l = exprOutputAsValue(leftVal);
      const r = exprOutputAsValue(rightVal);
      if (l === null || r === null) return false;
      return compareValuesWithCast(l, r, (a, b) => valueGt(a, b));
    }

    case 'GreaterThanOrEqual': {
      const l = exprOutputAsValue(leftVal);
      const r = exprOutputAsValue(rightVal);
      if (l === null || r === null) return false;
      return compareValuesWithCast(l, r, (a, b) => valueGe(a, b));
    }

    case 'LessThan': {
      const l = exprOutputAsValue(leftVal);
      const r = exprOutputAsValue(rightVal);
      if (l === null || r === null) return false;
      return compareValuesWithCast(l, r, (a, b) => valueLt(a, b));
    }

    case 'LessThanOrEqual': {
      const l = exprOutputAsValue(leftVal);
      const r = exprOutputAsValue(rightVal);
      if (l === null || r === null) return false;
      return compareValuesWithCast(l, r, (a, b) => valueLe(a, b));
    }

    case 'In': {
      const value = exprOutputAsValue(leftVal);
      const list = exprOutputAsList(rightVal);
      if (value === null || list === null) return false;
      return list.some((item) => {
        const v = exprOutputAsValue(item);
        if (v === null) return false;
        return compareValuesWithCast(value, v, (a, b) => valueEquals(a, b));
      });
    }

    case 'Between':
      // BETWEEN operator not yet supported
      return false;
  }
}
