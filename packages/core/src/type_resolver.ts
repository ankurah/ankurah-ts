// MIRRORS: ankurah/core/src/type_resolver.rs
//
// Type resolution for query AST preparation.
//
// The TypeResolver determines the ValueType for paths in queries, enabling proper
// literal conversion before execution. This is a temporary heuristic-based solution
// until Phase 3 schema metadata is implemented.
//
// Current heuristics:
// - Multi-step paths (e.g., `data.number`) → ValueType::Json (nested JSON traversal)
// - `id` field → ValueType::EntityId
// - Simple paths → infer from the literal being compared
//
// ## AST Mutation (Temporary)
//
// Until we have a proper MIR (Mid-level IR) tree, we temporarily mutate the AST
// in place via `prepare_predicate`. This converts literals to `Literal::Json`
// when comparing against JSON paths, ensuring type-aware comparison semantics
// match across Postgres, Sled, and in-memory filtering.

import { Expr, Literal, PathExpr, Predicate, Selection, ComparisonOperator } from '@ankurah/ankql';
import { ValueType } from './value/index.ts';
import { tryCastTo, valueFromLiteral, valueToLiteral } from './value/index.ts';
import type { Value } from './value/index.ts';

// ── TypeResolver ──────────────────────────────────────────────────────
// Rust: pub struct TypeResolver;
// derive(Debug, Clone, Default)

/**
 * Determines ValueType for paths in queries.
 *
 * TODO(Phase 3): Replace heuristics with proper schema lookup from System tables.
 */
export class TypeResolver {
  constructor() {}

  static new(): TypeResolver { return new TypeResolver(); }

  /**
   * Resolve the ValueType for a path expression.
   *
   * Returns:
   * - `ValueType.Json` for multi-step paths (nested JSON traversal)
   * - `ValueType.EntityId` for the "id" field
   * - `null` for simple paths (caller should use literal's inherent type)
   *
   * Rust: pub fn resolve_path(&self, path: &PathExpr) -> Option<ValueType>
   */
  resolvePath(path: PathExpr): ValueType | null {
    // Multi-step paths are JSON subfield traversals
    if (!path.isSimple()) {
      return ValueType.Json;
    }

    // The "id" field is always EntityId
    if (path.first() === 'id') {
      return ValueType.EntityId;
    }

    // For simple paths, return null to indicate the literal's type should be used
    return null;
  }

  /**
   * Get the ValueType for a literal.
   *
   * Rust: pub fn literal_type(literal: &Literal) -> ValueType
   */
  static literalType(literal: Literal): ValueType {
    switch (literal.type) {
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

  /**
   * Convert a Literal to a Json Literal if the comparison requires JSON semantics.
   *
   * Round-trips through Value.castTo for consistent behavior with the rest
   * of the casting system. This is temporary until we have a proper MIR tree.
   *
   * Rust: pub fn literal_to_json(literal: &Literal) -> Literal
   */
  static literalToJson(literal: Literal): Literal {
    const value: Value = valueFromLiteral(literal);
    const jsonValue = tryCastTo(value, ValueType.Json);
    if (jsonValue !== null) {
      return valueToLiteral(jsonValue);
    }
    // Fallback if cast fails (e.g., EntityId)
    return literal;
  }

  /**
   * Resolve the type for an expression (path or literal).
   *
   * Rust: fn resolve_expr_type(&self, expr: &Expr) -> Option<ValueType>
   */
  private resolveExprType(expr: Expr): ValueType | null {
    return expr.match({
      Path: (v) => this.resolvePath(v.path),
      Literal: (v) => TypeResolver.literalType(v.literal),
      Predicate: () => null,
      InfixExpr: () => null,
      ExprList: () => null,
      Placeholder: () => null,
    });
  }

  /**
   * Convert an expression's literal to the target type if needed.
   * Round-trips through Value.castTo for consistent casting behavior.
   *
   * Rust: fn convert_expr(&self, expr: Expr, target_type: Option<ValueType>) -> Expr
   */
  private convertExpr(expr: Expr, targetType: ValueType | null): Expr {
    if (targetType === null) return expr;

    return expr.match({
      Literal: (v) => {
        const value: Value = valueFromLiteral(v.literal);
        const casted = tryCastTo(value, targetType);
        if (casted !== null) {
          return Expr.Literal(valueToLiteral(casted));
        }
        // Fallback if cast fails
        return Expr.Literal(v.literal);
      },
      Path: () => expr,
      Predicate: () => expr,
      InfixExpr: () => expr,
      ExprList: () => expr,
      Placeholder: () => expr,
    });
  }

  /**
   * Resolve types in a selection, returning a new selection with converted literals.
   * Eventually this will return a TAST (Typed AST).
   *
   * Rust: pub fn resolve_selection_types(&self, selection: Selection) -> Selection
   */
  resolveSelectionTypes(selection: Selection): Selection {
    return new Selection(
      this.resolveTypes(selection.predicate),
      selection.orderBy,
      selection.limit,
    );
  }

  /**
   * Resolve types in a predicate, returning a new predicate with converted literals.
   *
   * Uses `resolvePath` to determine field types, then converts literals on the
   * other side of comparisons to match. Eventually this will return a TAST.
   *
   * Rust: pub fn resolve_types(&self, predicate: Predicate) -> Predicate
   */
  resolveTypes(predicate: Predicate): Predicate {
    return predicate.match({
      Comparison: (v) => {
        // Look up types from paths
        const leftType = this.resolveExprType(v.left);
        const rightType = this.resolveExprType(v.right);

        // Convert literals based on the type from the other side
        const newLeft = this.convertExpr(v.left, rightType);
        const newRight = this.convertExpr(v.right, leftType);

        return Predicate.Comparison(newLeft, v.operator, newRight);
      },
      And: (v) => Predicate.And(this.resolveTypes(v.left), this.resolveTypes(v.right)),
      Or: (v) => Predicate.Or(this.resolveTypes(v.left), this.resolveTypes(v.right)),
      Not: (v) => Predicate.Not(this.resolveTypes(v.predicate)),
      // These don't need type resolution
      IsNull: () => predicate,
      True: () => predicate,
      False: () => predicate,
      Placeholder: () => predicate,
    });
  }
}
