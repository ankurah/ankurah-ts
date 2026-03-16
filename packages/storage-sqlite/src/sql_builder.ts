// MIRRORS: ankurah/storage/sqlite/src/sql_builder.rs

import { type Selection, type OrderByItem, Predicate, Expr, Literal, ComparisonOperator } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';
import { SqliteError } from './error.ts';

// ── SqlGenerationError ────────────────────────────────────────────────

export class SqlGenerationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SqlGenerationError';
  }

  static placeholderFound(): SqlGenerationError {
    return new SqlGenerationError('Placeholder found in predicate - placeholders should be replaced before predicate processing');
  }

  static unsupportedExpression(desc: string): SqlGenerationError {
    return new SqlGenerationError(`Unsupported expression type: ${desc}`);
  }

  static unsupportedOperator(desc: string): SqlGenerationError {
    return new SqlGenerationError(`Unsupported operator: ${desc}`);
  }
}

// ── SplitPredicate ────────────────────────────────────────────────────

/**
 * Result of splitting a predicate for SQLite execution.
 *
 * Rust: `pub struct SplitPredicate { sql_predicate, remaining_predicate }`
 */
export class SplitPredicate {
  readonly sqlPredicate: Predicate;
  readonly remainingPredicate: Predicate;

  constructor(sqlPredicate: Predicate, remainingPredicate: Predicate) {
    this.sqlPredicate = sqlPredicate;
    this.remainingPredicate = remainingPredicate;
  }

  needsPostFilter(): boolean {
    return !this.remainingPredicate.is('True');
  }
}

// ── splitPredicateForSqlite ───────────────────────────────────────────

/** Split a predicate into parts that can be pushed down to SQLite vs evaluated post-fetch. */
export function splitPredicateForSqlite(predicate: Predicate): SplitPredicate {
  const [sqlPred, remainingPred] = splitPredicateRecursive(predicate);
  return new SplitPredicate(sqlPred, remainingPred);
}

function splitPredicateRecursive(predicate: Predicate): [Predicate, Predicate] {
  return predicate.match<[Predicate, Predicate]>({
    Comparison: (v) => {
      if (canPushdownComparison(v.left, v.right)) {
        return [predicate, Predicate.True()];
      }
      return [Predicate.True(), predicate];
    },
    And: (v) => {
      const [leftSql, leftRemaining] = splitPredicateRecursive(v.left);
      const [rightSql, rightRemaining] = splitPredicateRecursive(v.right);

      const sqlPred = leftSql.is('True') && rightSql.is('True')
        ? Predicate.True()
        : leftSql.is('True') ? rightSql
        : rightSql.is('True') ? leftSql
        : Predicate.And(leftSql, rightSql);

      const remainingPred = leftRemaining.is('True') && rightRemaining.is('True')
        ? Predicate.True()
        : leftRemaining.is('True') ? rightRemaining
        : rightRemaining.is('True') ? leftRemaining
        : Predicate.And(leftRemaining, rightRemaining);

      return [sqlPred, remainingPred];
    },
    Or: (v) => {
      const [leftSql, leftRemaining] = splitPredicateRecursive(v.left);
      const [rightSql, rightRemaining] = splitPredicateRecursive(v.right);

      if (leftRemaining.is('True') && rightRemaining.is('True')) {
        return [predicate, Predicate.True()];
      }

      const sqlPred = leftSql.is('True') && rightSql.is('True')
        ? Predicate.True()
        : leftSql.is('True') ? rightSql
        : rightSql.is('True') ? leftSql
        : Predicate.Or(leftSql, rightSql);

      return [sqlPred, predicate];
    },
    Not: (v) => {
      const [innerSql, innerRemaining] = splitPredicateRecursive(v.predicate);
      if (innerRemaining.is('True')) {
        return [Predicate.Not(innerSql), Predicate.True()];
      }
      return [Predicate.True(), predicate];
    },
    IsNull: (v) => {
      if (canPushdownExpr(v.expr)) {
        return [predicate, Predicate.True()];
      }
      return [Predicate.True(), predicate];
    },
    True: () => [Predicate.True(), Predicate.True()],
    False: () => [Predicate.False(), Predicate.True()],
    Placeholder: () => [Predicate.True(), predicate],
  });
}

function canPushdownComparison(left: Expr, right: Expr): boolean {
  return canPushdownExpr(left) && canPushdownExpr(right);
}

function canPushdownExpr(expr: Expr): boolean {
  return expr.match<boolean>({
    Literal: () => true,
    Path: (v) => v.path.steps.length > 0,
    ExprList: (v) => v.exprs.every(canPushdownExpr),
    Predicate: () => false,
    InfixExpr: () => false,
    Placeholder: () => false,
  });
}

// ── SqlParam ──────────────────────────────────────────────────────────

/** Bind parameter type for SQLite drivers. */
export type SqlParam = string | number | Uint8Array | null;

// ── SqlBuilder ────────────────────────────────────────────────────────

/**
 * SQL builder for SQLite queries.
 *
 * Rust: `pub struct SqlBuilder { sql, params, fields, table_name }`
 * Divergence: Uses SqlParam instead of rusqlite::types::Value [E16].
 */
export class SqlBuilder {
  private sql = '';
  private params: SqlParam[] = [];
  private fields: string[] = [];
  private tableName: string | null = null;

  static new(): SqlBuilder {
    return new SqlBuilder();
  }

  static withFields(fields: string[]): SqlBuilder {
    const b = new SqlBuilder();
    b.fields = [...fields];
    return b;
  }

  setTableName(name: string): this {
    this.tableName = name;
    return this;
  }

  private pushSql(s: string): void {
    this.sql += s;
  }

  private pushParam(value: SqlParam): void {
    this.sql += '?';
    this.params.push(value);
  }

  build(): [string, SqlParam[]] {
    if (this.fields.length === 0 || this.tableName === null) {
      return [this.sql, this.params];
    }

    const fieldsClause = this.fields
      .map((f) => `"${f.replace(/"/g, '""')}"`)
      .join(', ');
    const table = this.tableName.replace(/"/g, '""');
    const fullSql = `SELECT ${fieldsClause} FROM "${table}" WHERE ${this.sql}`;
    return [fullSql, this.params];
  }

  buildWhereClause(): [string, SqlParam[]] {
    return [this.sql, this.params];
  }

  expr(e: Expr): void {
    e.match({
      Placeholder: () => { throw SqlGenerationError.placeholderFound(); },
      Literal: (v) => { this.literal(v.literal); },
      Path: (v) => {
        const path = v.path;
        if (path.isSimple()) {
          const escaped = path.first().replace(/"/g, '""');
          this.pushSql(`"${escaped}"`);
        } else {
          // Multi-step path: JSONB traversal using json_extract()
          const first = path.first().replace(/"/g, '""');
          if (path.steps.length === 2) {
            const jsonPath = `$.${path.steps[1].replace(/'/g, "''")}`;
            this.pushSql(`json_extract("${first}", '${jsonPath}')`);
          } else {
            const jsonPath = `$.${path.steps.slice(1).map((s) => s.replace(/'/g, "''")).join('.')}`;
            this.pushSql(`json_extract("${first}", '${jsonPath}')`);
          }
        }
      },
      ExprList: (v) => {
        this.pushSql('(');
        for (let i = 0; i < v.exprs.length; i++) {
          if (i > 0) this.pushSql(', ');
          this.expr(v.exprs[i]);
        }
        this.pushSql(')');
      },
      Predicate: () => { throw SqlGenerationError.unsupportedExpression('Predicate expression'); },
      InfixExpr: () => { throw SqlGenerationError.unsupportedExpression('InfixExpr'); },
    });
  }

  private literal(lit: Literal): void {
    lit.match({
      String: (v) => { this.pushParam(v.value); },
      I64: (v) => { this.pushParam(Number(v.value)); },
      F64: (v) => { this.pushParam(v.value); },
      Bool: (v) => { this.pushParam(v.value ? 1 : 0); },
      I16: (v) => { this.pushParam(v.value); },
      I32: (v) => { this.pushParam(v.value); },
      EntityId: (v) => { this.pushParam(EntityId.fromBytes(v.value).toBase64()); },
      Object: (v) => { this.pushParam(v.value); },
      Binary: (v) => { this.pushParam(v.value); },
      Json: (v) => {
        const json = v.value;
        if (typeof json === 'string') {
          this.pushParam(json);
        } else if (typeof json === 'number') {
          this.pushParam(json);
        } else if (typeof json === 'boolean') {
          this.pushParam(json ? 1 : 0);
        } else if (json === null) {
          this.pushParam(null);
        } else {
          this.pushParam(JSON.stringify(json));
        }
      },
    });
  }

  comparisonOp(op: ComparisonOperator): void {
    this.pushSql(comparisonOpToSql(op));
  }

  predicate(pred: Predicate): void {
    pred.match({
      Comparison: (v) => {
        this.expr(v.left);
        this.pushSql(' ');
        this.comparisonOp(v.operator);
        this.pushSql(' ');
        this.expr(v.right);
      },
      And: (v) => {
        this.predicate(v.left);
        this.pushSql(' AND ');
        this.predicate(v.right);
      },
      Or: (v) => {
        this.pushSql('(');
        this.predicate(v.left);
        this.pushSql(' OR ');
        this.predicate(v.right);
        this.pushSql(')');
      },
      Not: (v) => {
        this.pushSql('NOT (');
        this.predicate(v.predicate);
        this.pushSql(')');
      },
      IsNull: (v) => {
        this.expr(v.expr);
        this.pushSql(' IS NULL');
      },
      True: () => { this.pushSql('1=1'); },
      False: () => { this.pushSql('1=0'); },
      Placeholder: () => { throw SqlGenerationError.placeholderFound(); },
    });
  }

  selection(sel: Selection): void {
    this.predicate(sel.predicate);

    if (sel.orderBy !== null && sel.orderBy.length > 0) {
      this.pushSql(' ORDER BY ');
      for (let i = 0; i < sel.orderBy.length; i++) {
        if (i > 0) this.pushSql(', ');
        this.orderByItem(sel.orderBy[i]);
      }
    }

    if (sel.limit !== null) {
      this.pushSql(` LIMIT ${sel.limit}`);
    }
  }

  orderByItem(orderBy: OrderByItem): void {
    if (orderBy.path.isSimple()) {
      const escaped = orderBy.path.first().replace(/"/g, '""');
      this.pushSql(`"${escaped}"`);
    } else {
      const first = orderBy.path.first().replace(/"/g, '""');
      this.pushSql(`"${first}"`);
      for (let i = 1; i < orderBy.path.steps.length; i++) {
        const escaped = orderBy.path.steps[i].replace(/'/g, "''");
        this.pushSql(`->'${escaped}'`);
      }
    }

    if (orderBy.direction.is('Asc')) {
      this.pushSql(' ASC');
    } else {
      this.pushSql(' DESC');
    }
  }
}

// ── Helper ────────────────────────────────────────────────────────────

function comparisonOpToSql(op: ComparisonOperator): string {
  return op.match<string>({
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
