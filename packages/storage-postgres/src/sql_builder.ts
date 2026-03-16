// MIRRORS: ankurah/storage/postgres/src/sql_builder.rs

import {
  ComparisonOperator,
  Expr,
  Literal,
  OrderByItem,
  OrderDirection,
  Predicate,
  Selection,
  PathExpr,
  parseSelection,
} from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';
import { RetrievalError } from '@ankurah/core';

// ── SqlGenerationError ───────────────────────────────────────────────

export type SqlGenerationErrorKind =
  | 'PlaceholderFound'
  | 'UnsupportedExpression'
  | 'UnsupportedOperator'
  | 'IncompleteConfiguration';

export class SqlGenerationError extends Error {
  readonly kind: SqlGenerationErrorKind;

  constructor(kind: SqlGenerationErrorKind, message: string) {
    super(message);
    this.name = 'SqlGenerationError';
    this.kind = kind;
  }

  static placeholderFound(): SqlGenerationError {
    return new SqlGenerationError(
      'PlaceholderFound',
      'Placeholder found in predicate - placeholders should be replaced before predicate processing',
    );
  }

  static unsupportedExpression(detail: string): SqlGenerationError {
    return new SqlGenerationError('UnsupportedExpression', `Unsupported expression type: ${detail}`);
  }

  static unsupportedOperator(detail: string): SqlGenerationError {
    return new SqlGenerationError('UnsupportedOperator', `Unsupported operator: ${detail}`);
  }

  static incompleteConfiguration(): SqlGenerationError {
    return new SqlGenerationError(
      'IncompleteConfiguration',
      'SqlBuilder requires both fields and table_name to be set for complete SELECT generation, or neither for WHERE-only mode',
    );
  }
}

// impl From<SqlGenerationError> for RetrievalError
// Divergence: TS uses static factory, not From trait [E7]

export function retrievalErrorFromSqlError(err: SqlGenerationError): RetrievalError {
  return RetrievalError.storageError(err);
}

// ── SplitPredicate ───────────────────────────────────────────────────

/// Result of splitting a predicate for PostgreSQL execution.
///
/// "Pushdown" refers to moving predicate evaluation from the application layer
/// down to the database layer. Some predicates can be translated to SQL and
/// executed by PostgreSQL (pushdown), while others must be evaluated in TS
/// after fetching results (e.g., future features like Ref traversal).
export class SplitPredicate {
  /// Predicate that can be pushed down to PostgreSQL WHERE clause
  readonly sqlPredicate: Predicate;
  /// Predicate that must be evaluated in TS after fetching (Predicate::True if nothing remains)
  readonly remainingPredicate: Predicate;

  constructor(sqlPredicate: Predicate, remainingPredicate: Predicate) {
    this.sqlPredicate = sqlPredicate;
    this.remainingPredicate = remainingPredicate;
  }

  /// Check if there's any remaining predicate that needs post-filtering
  needsPostFilter(): boolean {
    return !this.remainingPredicate.is('True');
  }
}

// ── split_predicate_for_postgres ─────────────────────────────────────

/// Split a predicate into parts that can be pushed down to PostgreSQL vs evaluated post-fetch.
///
/// **Pushdown-capable** (translated to SQL):
/// - Simple column comparisons (single-step paths like `name = 'value'`)
/// - JSONB path comparisons (multi-step paths like `data.field = 'value'`)
/// - AND/OR/NOT combinations of pushdown-capable predicates
/// - IS NULL, TRUE, FALSE
///
/// **Requires post-filtering** (evaluated in TS):
/// - Future: Ref traversals, complex expressions
export function splitPredicateForPostgres(predicate: Predicate): SplitPredicate {
  const [sqlPred, remainingPred] = splitPredicateRecursive(predicate);
  return new SplitPredicate(sqlPred, remainingPred);
}

/// Recursively split a predicate into (pushdown, remaining) parts.
function splitPredicateRecursive(predicate: Predicate): [Predicate, Predicate] {
  return predicate.match({
    // Leaf predicates - check if they support pushdown
    Comparison: (v) => {
      if (canPushdownComparison(v.left, v.right)) {
        return [predicate, Predicate.True()] as [Predicate, Predicate];
      } else {
        // Can't pushdown - keep for post-filter
        return [Predicate.True(), predicate] as [Predicate, Predicate];
      }
    },

    // AND: can split - pushdown what we can, keep the rest
    And: (v) => {
      const [leftSql, leftRemaining] = splitPredicateRecursive(v.left);
      const [rightSql, rightRemaining] = splitPredicateRecursive(v.right);

      let sqlPred: Predicate;
      if (leftSql.is('True') && rightSql.is('True')) {
        sqlPred = Predicate.True();
      } else if (leftSql.is('True')) {
        sqlPred = rightSql;
      } else if (rightSql.is('True')) {
        sqlPred = leftSql;
      } else {
        sqlPred = Predicate.And(leftSql, rightSql);
      }

      let remainingPred: Predicate;
      if (leftRemaining.is('True') && rightRemaining.is('True')) {
        remainingPred = Predicate.True();
      } else if (leftRemaining.is('True')) {
        remainingPred = rightRemaining;
      } else if (rightRemaining.is('True')) {
        remainingPred = leftRemaining;
      } else {
        remainingPred = Predicate.And(leftRemaining, rightRemaining);
      }

      return [sqlPred, remainingPred] as [Predicate, Predicate];
    },

    // OR: if any branch can't be fully pushed down, keep the whole OR for post-filter
    // (but still pushdown what we can to reduce row count)
    Or: (v) => {
      const [leftSql, leftRemaining] = splitPredicateRecursive(v.left);
      const [rightSql, rightRemaining] = splitPredicateRecursive(v.right);

      // If both branches fully support pushdown, pushdown the whole OR
      if (leftRemaining.is('True') && rightRemaining.is('True')) {
        return [predicate, Predicate.True()] as [Predicate, Predicate];
      } else {
        // Partial pushdown - still send what we can to reduce rows,
        // but must also post-filter with the full OR
        let sqlPred: Predicate;
        if (leftSql.is('True') && rightSql.is('True')) {
          sqlPred = Predicate.True();
        } else if (leftSql.is('True')) {
          sqlPred = rightSql;
        } else if (rightSql.is('True')) {
          sqlPred = leftSql;
        } else {
          sqlPred = Predicate.Or(leftSql, rightSql);
        }
        return [sqlPred, predicate] as [Predicate, Predicate];
      }
    },

    // NOT: pushdown if inner supports pushdown
    Not: (v) => {
      const [innerSql, innerRemaining] = splitPredicateRecursive(v.predicate);
      if (innerRemaining.is('True')) {
        return [Predicate.Not(innerSql), Predicate.True()] as [Predicate, Predicate];
      } else {
        // Can't pushdown the NOT - keep whole thing for post-filter
        return [Predicate.True(), predicate] as [Predicate, Predicate];
      }
    },

    // IS NULL - pushdown if expression supports pushdown
    IsNull: (v) => {
      if (canPushdownExpr(v.expr)) {
        return [predicate, Predicate.True()] as [Predicate, Predicate];
      } else {
        return [Predicate.True(), predicate] as [Predicate, Predicate];
      }
    },

    True: () => [Predicate.True(), Predicate.True()] as [Predicate, Predicate],
    False: () => [Predicate.False(), Predicate.True()] as [Predicate, Predicate],
    Placeholder: () => [Predicate.True(), predicate] as [Predicate, Predicate], // Shouldn't happen, but be safe
  });
}

// ── can_pushdown helpers ─────────────────────────────────────────────

/// Check if a comparison can be pushed down to PostgreSQL.
function canPushdownComparison(left: Expr, right: Expr): boolean {
  return canPushdownExpr(left) && canPushdownExpr(right);
}

/// Check if an expression can be pushed down to PostgreSQL SQL.
///
/// Returns true if the expression can be translated to valid PostgreSQL syntax.
/// Currently supports:
/// - Literals (strings, numbers, booleans, etc.)
/// - Simple column paths (`name`) - regular column reference
/// - Multi-step paths (`data.field`) - JSONB traversal via `->` and `->>`
/// - Expression lists (for IN clauses)
///
/// NOT pushdown-capable (will be post-filtered in TS):
/// - Nested predicates as expressions
/// - Infix expressions (not yet implemented)
/// - Placeholders (should be replaced before we get here)
///
/// HACK: We currently infer "JSON property" from multi-step paths. This works for Phase 1
/// where only Json properties support nested traversal.
function canPushdownExpr(expr: Expr): boolean {
  return expr.match({
    Literal: () => true,
    Path: (v) => {
      // All paths are currently pushdown-capable:
      // - Single-step: regular column reference
      // - Multi-step: JSONB traversal (inferred as Json property for now)
      //
      // HACK: We assume multi-step paths are Json properties.
      return v.path.steps.length > 0;
    },
    ExprList: (v) => v.exprs.every(canPushdownExpr),
    Predicate: () => false,     // Nested predicates - not supported in SQL expressions
    InfixExpr: () => false,     // Not yet supported
    Placeholder: () => false,   // Should be replaced before we get here
  });
}

// ── SqlExpr ──────────────────────────────────────────────────────────

// Divergence: Rust uses `Box<dyn ToSql + Send + Sync>` for type-erased PG args.
// TS represents arguments as unknown — the actual PG client binding handles
// type-specific serialization at query time [E8].

export type SqlExpr =
  | { type: 'Sql'; value: string }
  | { type: 'Argument'; value: unknown };

// ── SqlBuilder ───────────────────────────────────────────────────────

export class SqlBuilder {
  private expressions: SqlExpr[] = [];
  private _fields: string[] = [];
  private _tableName: string | null = null;

  constructor() {}

  static withFields(fields: string[]): SqlBuilder {
    const builder = new SqlBuilder();
    builder._fields = [...fields];
    return builder;
  }

  tableName(name: string): this {
    this._tableName = name;
    return this;
  }

  push(expr: SqlExpr): void {
    this.expressions.push(expr);
  }

  arg(value: unknown): void {
    this.push({ type: 'Argument', value });
  }

  sql(s: string): void {
    this.push({ type: 'Sql', value: s });
  }

  build(): { sql: string; args: unknown[] } {
    let counter = 1;
    let whereClause = '';
    const args: unknown[] = [];

    // Build WHERE clause from expressions
    for (const expr of this.expressions) {
      if (expr.type === 'Argument') {
        whereClause += `$${counter}`;
        args.push(expr.value);
        counter += 1;
      } else {
        whereClause += expr.value;
      }
    }

    // Build complete SELECT statement - fields and table are required
    if (this._fields.length === 0 || this._tableName === null) {
      throw SqlGenerationError.incompleteConfiguration();
    }

    const fieldsClause = this._fields
      .map((field) => `"${field.replace(/"/g, '""')}"`)
      .join(', ');
    const table = this._tableName;
    const sqlStr = `SELECT ${fieldsClause} FROM "${table.replace(/"/g, '""')}" WHERE ${whereClause}`;

    return { sql: sqlStr, args };
  }

  buildWhereClause(): { sql: string; args: unknown[] } {
    let counter = 1;
    let whereClause = '';
    const args: unknown[] = [];

    // Build WHERE clause from expressions
    for (const expr of this.expressions) {
      if (expr.type === 'Argument') {
        whereClause += `$${counter}`;
        args.push(expr.value);
        counter += 1;
      } else {
        whereClause += expr.value;
      }
    }

    return { sql: whereClause, args };
  }

  // --- AST flattening ---

  expr(expr: Expr): void {
    expr.match({
      Placeholder: () => { throw SqlGenerationError.placeholderFound(); },
      Literal: (v) => {
        this.emitLiteral(v.literal);
      },
      Path: (v) => {
        const path = v.path;
        if (path.isSimple()) {
          // Single-step path: regular column reference "column_name"
          const escaped = path.first().replace(/"/g, '""');
          this.sql(`"${escaped}"`);
        } else {
          // Multi-step path: JSONB traversal "column"->'nested'->'path'
          // Use -> for ALL steps to preserve JSONB type for proper comparison semantics.
          const first = path.first().replace(/"/g, '""');
          this.sql(`"${first}"`);

          for (let i = 1; i < path.steps.length; i++) {
            const escaped = path.steps[i].replace(/'/g, "''");
            // Always use -> to keep as JSONB (not ->> which extracts as text)
            this.sql(`->'${escaped}'`);
          }
        }
      },
      ExprList: (v) => {
        this.sql('(');
        for (let i = 0; i < v.exprs.length; i++) {
          if (i > 0) {
            this.sql(', ');
          }
          const item = v.exprs[i];
          if (item.is('Placeholder')) {
            throw SqlGenerationError.placeholderFound();
          } else if (item.is('Literal')) {
            const litVal = (item.value as { literal: Literal }).literal;
            this.emitLiteral(litVal);
          } else {
            throw SqlGenerationError.unsupportedExpression(
              'Only literal expressions and placeholders are supported in IN lists',
            );
          }
        }
        this.sql(')');
      },
      Predicate: () => {
        throw SqlGenerationError.unsupportedExpression(
          'Only literal, identifier, and list expressions are supported',
        );
      },
      InfixExpr: () => {
        throw SqlGenerationError.unsupportedExpression(
          'Only literal, identifier, and list expressions are supported',
        );
      },
    });
  }

  /// Emit a literal as a parameterized argument
  private emitLiteral(lit: Literal): void {
    lit.match({
      String: (v) => this.arg(v.value),
      I64: (v) => this.arg(v.value),
      F64: (v) => this.arg(v.value),
      Bool: (v) => this.arg(v.value),
      I16: (v) => this.arg(v.value),
      I32: (v) => this.arg(v.value),
      EntityId: (v) => {
        // Divergence: Rust calls EntityId::from_ulid().to_base64(). TS stores raw bytes in the Literal [E8].
        this.arg(EntityId.fromBytes(v.value).toBase64());
      },
      Object: (v) => this.arg(v.value),
      Binary: (v) => this.arg(v.value),
      Json: (v) => this.arg(v.value),
    });
  }

  /// Emit a literal expression with ::jsonb cast for proper JSONB comparison semantics.
  /// This ensures that comparisons like `"data"->'count' > '10'::jsonb` work correctly
  /// with PostgreSQL's type-aware JSONB comparison (numeric vs lexicographic).
  exprAsJsonb(expr: Expr): void {
    if (expr.is('Literal')) {
      const lit = (expr.value as { literal: Literal }).literal;
      // For literals, we need to cast to jsonb
      lit.match({
        String: (v) => {
          // String literals need to be JSON strings: '"value"'::jsonb
          const jsonEscaped = v.value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
          const sqlEscaped = `"${jsonEscaped}"`.replace(/'/g, "''");
          this.sql(`'${sqlEscaped}'::jsonb`);
        },
        I64: (v) => this.sql(`'${v.value}'::jsonb`),
        F64: (v) => this.sql(`'${v.value}'::jsonb`),
        Bool: (v) => this.sql(`'${v.value}'::jsonb`),
        I16: (v) => this.sql(`'${v.value}'::jsonb`),
        I32: (v) => this.sql(`'${v.value}'::jsonb`),
        // EntityId and binary types don't make sense as JSONB
        EntityId: () => { this.expr(expr); },
        Object: () => { this.expr(expr); },
        Binary: () => { this.expr(expr); },
        // JSON literal is already properly typed
        Json: (v) => this.sql(`'${JSON.stringify(v.value)}'::jsonb`),
      });
    } else {
      // For non-literals, just emit normally (they're already JSONB paths or complex expressions)
      this.expr(expr);
    }
  }

  comparisonOp(op: ComparisonOperator): void {
    this.sql(comparisonOpToSql(op));
  }

  predicate(predicate: Predicate): void {
    predicate.match({
      Comparison: (v) => {
        // Check if either side is a JSONB path (multi-step path)
        const leftIsJsonb = v.left.is('Path') && !(v.left.value as { path: PathExpr }).path.isSimple();
        const rightIsJsonb = v.right.is('Path') && !(v.right.value as { path: PathExpr }).path.isSimple();

        this.expr(v.left);
        this.sql(' ');
        this.comparisonOp(v.operator);
        this.sql(' ');

        if (leftIsJsonb && v.right.is('Literal')) {
          // Comparing JSONB path to literal: cast literal to jsonb
          this.exprAsJsonb(v.right);
        } else if (rightIsJsonb && v.left.is('Literal')) {
          // Comparing literal to JSONB path: cast literal to jsonb
          this.exprAsJsonb(v.right);
        } else {
          this.expr(v.right);
        }
      },
      And: (v) => {
        this.predicate(v.left);
        this.sql(' AND ');
        this.predicate(v.right);
      },
      Or: (v) => {
        this.sql('(');
        this.predicate(v.left);
        this.sql(' OR ');
        this.predicate(v.right);
        this.sql(')');
      },
      Not: (v) => {
        this.sql('NOT (');
        this.predicate(v.predicate);
        this.sql(')');
      },
      IsNull: (v) => {
        this.expr(v.expr);
        this.sql(' IS NULL');
      },
      True: () => {
        this.sql('TRUE');
      },
      False: () => {
        this.sql('FALSE');
      },
      Placeholder: () => {
        throw SqlGenerationError.placeholderFound();
      },
    });
  }

  selection(selection: Selection): void {
    // Add the predicate (WHERE clause)
    this.predicate(selection.predicate);

    // Add ORDER BY clause if present
    if (selection.orderBy) {
      this.sql(' ORDER BY ');
      for (let i = 0; i < selection.orderBy.length; i++) {
        if (i > 0) {
          this.sql(', ');
        }
        this.orderByItem(selection.orderBy[i]);
      }
    }

    // Add LIMIT clause if present
    if (selection.limit !== null) {
      this.sql(' LIMIT ');
      this.arg(BigInt(selection.limit)); // PostgreSQL expects i64 for LIMIT
    }
  }

  orderByItem(orderBy: OrderByItem): void {
    // Generate the path expression
    for (let i = 0; i < orderBy.path.steps.length; i++) {
      if (i > 0) {
        this.sql('.');
      }
      // Escape any existing quotes in the step by doubling them
      const escapedStep = orderBy.path.steps[i].replace(/"/g, '""');
      this.sql(`"${escapedStep}"`);
    }

    // Add the direction
    if (orderBy.direction.is('Asc')) {
      this.sql(' ASC');
    } else {
      this.sql(' DESC');
    }
  }
}

// ── comparison_op_to_sql ─────────────────────────────────────────────

function comparisonOpToSql(op: ComparisonOperator): string {
  return op.match({
    Equal: () => '=',
    NotEqual: () => '<>',
    GreaterThan: () => '>',
    GreaterThanOrEqual: () => '>=',
    LessThan: () => '<',
    LessThanOrEqual: () => '<=',
    In: () => 'IN',
    Between: () => { throw SqlGenerationError.unsupportedOperator('BETWEEN operator is not yet supported'); },
  });
}

// =========================================================================
// Tests
// MIRRORS: ankurah/storage/postgres/src/sql_builder.rs #[cfg(test)]
// =========================================================================

if (import.meta.main) {
  // Tests are in sql_builder.test.ts
}
