// MIRRORS: ankurah/storage/sqlite/src/sql_builder.rs
import { Struct, Enum, Result } from '@ankurah/base';
import { ComparisonOperator, Expr, Literal, OrderByItem, Predicate, Selection } from '@ankurah/ankql';
import { EntityId, Comparison, Json, Value } from '@ankurah/core';
import { SqliteError } from './error';
import { EntityId } from '@ankurah/proto';

export class SplitPredicate extends Struct {
  readonly sqlPredicate: Predicate;
  readonly remainingPredicate: Predicate;

  constructor(sqlPredicate: Predicate, remainingPredicate: Predicate) {
    super();
    this.sqlPredicate = sqlPredicate;
    this.remainingPredicate = remainingPredicate;
  }

  needsPostFilter(): boolean {
    return !this.remainingPredicate.is('True');
  }

  clone(): SplitPredicate {
    return new SplitPredicate(this.sqlPredicate.clone(), this.remainingPredicate.clone());
  }

  debug(): string {
    return `SplitPredicate { sqlPredicate: ${this.sqlPredicate.debug()}, remainingPredicate: ${this.remainingPredicate.debug()} }`;
  }
}

export class SqlBuilder extends Struct {
  sql: string;
  params: Value[];
  fields: string[];
  tableName: string | null;

  constructor(sql: string, params: Value[], fields: string[], tableName: string | null) {
    super();
    this.sql = sql;
    this.params = params;
    this.fields = fields;
    this.tableName = tableName;
  }

  static new(): SqlBuilder {
    return new SqlBuilder('', [], [], null);
  }

  static withFields<T extends Into>(fields: T[]): SqlBuilder {
    return new SqlBuilder('', [], [...fields].map((f) => f), null);
  }

  tableName(name: string): SqlBuilder {
    this.tableName = name;
    return this;
  }

  pushSql(s: string): void {
    this.sql += s;
  }

  pushParam(value: Value): void {
    this.sql += '?';
    this.params.push(value);
  }

  build(): Result<[string, Value[]], SqlGenerationError> {
    try {
      if (this.fields.length === 0 || (this.tableName == null)) {
        return Result.Ok([this.sql, this.params]);
      }
      const fieldsClause = [...this.fields].map((field) => `"${field.replace('"', '""')}"`).join(', ');
      const table = (this.tableName ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
      const sql = `SELECT ${fieldsClause} FROM "${table.replace('"', '""')}" WHERE ${this.sql}`;
      return Result.Ok([sql, this.params]);
    } finally {
      this.drop();
    }
  }

  buildWhereClause(): [string, Value[]] {
    try {
      return [this.sql, this.params];
    } finally {
      this.drop();
    }
  }

  expr(expr: Expr): Result<void, SqlGenerationError> {
    const _m1 = expr.match<any>({
      Placeholder: () => {
        return { $jump: 'return', $value: Result.Err(new SqlGenerationError('PlaceholderFound', {})) }
      },
      Literal: (v) => {
        const lit = v._0;
        this.literal(lit);
      },
      Path: (v) => {
        const path = v._0;
        if (path.isSimple()) {
          const escaped = path.first().replace('"', '""');
          this.pushSql(`"${escaped}"`);
        } else {
          const first = path.first().replace('"', '""');
          const jsonPath = (path.steps.length === 2 ? `$.${path.steps[1].replace('\'', '\'\'')}` : `$.${[...path.steps].slice(1).map((s) => s.replace('\'', '\'\'')).join('.')}`);
          this.pushSql(`json_extract("${first}", '${jsonPath}')`);
        }
      },
      ExprList: (v) => {
        const exprs = v._0;
        this.pushSql('(');
        for (const [i, expr] of [...exprs].entries()) {
          if (i > 0) {
            this.pushSql(', ');
          }
          const _r0 = this.expr(expr);
          if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
          _r0.drop();
        }
        this.pushSql(')');
      },
      Predicate: () => {
        return { $jump: 'return', $value: Result.Err(new SqlGenerationError('UnsupportedExpression', { _0: 'Only literal, path, and list expressions are supported' })) }
      },
      InfixExpr: () => {
        return { $jump: 'return', $value: Result.Err(new SqlGenerationError('UnsupportedExpression', { _0: 'Only literal, path, and list expressions are supported' })) }
      },
    });
    if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
    return Result.Ok([]);
  }

  literal(lit: Literal): void {
    return lit.match({
      String: (v) => {
        const s = v._0;
        this.pushParam(new rusqlite.types.Value('Text', { _0: s }));
      },
      I64: (v) => {
        const i = v._0;
        this.pushParam(new rusqlite.types.Value('Integer', { _0: i }));
      },
      F64: (v) => {
        const f = v._0;
        this.pushParam(new rusqlite.types.Value('Real', { _0: f }));
      },
      Bool: (v) => {
        const b = v._0;
        this.pushParam(new rusqlite.types.Value('Integer', { _0: (b ? 1 : 0) }));
      },
      I16: (v) => {
        const i = v._0;
        this.pushParam(new rusqlite.types.Value('Integer', { _0: BigInt(i) }));
      },
      I32: (v) => {
        const i = v._0;
        this.pushParam(new rusqlite.types.Value('Integer', { _0: BigInt(i) }));
      },
      EntityId: (v) => {
        const ulid = v._0;
        this.pushParam(new rusqlite.types.Value('Text', { _0: EntityId.fromUlid(ulid).toBase64() }));
      },
      Object: (v) => {
        const bytes = v._0;
        this.pushParam(new rusqlite.types.Value('Blob', { _0: bytes.clone() }));
      },
      Binary: (v) => {
        const bytes = v._0;
        this.pushParam(new rusqlite.types.Value('Blob', { _0: bytes.clone() }));
      },
      Json: (v) => {
        const json = v._0;
        json.match({
          String: (v) => {
            const s = v._0;
            this.pushParam(new rusqlite.types.Value('Text', { _0: s }));
          },
          Number: (v) => {
            const n = v._0;
            {
              const _v1 = n.asI64();
              if (_v1 != null) {
                const i = _v1;
                this.pushParam(new rusqlite.types.Value('Integer', { _0: i }));
              } else {
              const _v = n.asF64();
              if (_v != null) {
                const f = _v;
                this.pushParam(new rusqlite.types.Value('Real', { _0: f }));
              } else {
              this.pushParam(new rusqlite.types.Value('Text', { _0: n.toString() }));
            }
            }
            }
          },
          Bool: (v) => {
            const b = v._0;
            this.pushParam(new rusqlite.types.Value('Integer', { _0: (b ? 1 : 0) }));
          },
          Null: () => {
            this.pushParam(rusqlite.types.Value.Null);
          },
          Array: () => {
            this.pushParam(new rusqlite.types.Value('Text', { _0: json.toString() }));
          },
          Object: () => {
            this.pushParam(new rusqlite.types.Value('Text', { _0: json.toString() }));
          },
        });
      },
    });
  }

  comparisonOp(op: ComparisonOperator): Result<void, SqlGenerationError> {
    const _r0 = comparisonOpToSql(op);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    this.pushSql(_r0.unwrap());
    return Result.Ok([]);
  }

  predicate(predicate: Predicate): Result<void, SqlGenerationError> {
    const _m9 = predicate.match<any>({
      Comparison: (v) => {
        const left = v.left;
        const operator = v.operator;
        const right = v.right;
        const _r0 = this.expr(left);
        if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
        _r0.drop();
        this.pushSql(' ');
        const _r1 = this.comparisonOp(operator);
        if (_r1.isErr()) return { $jump: 'return', $value: Result.Err(_r1.unwrapErr()) };
        _r1.drop();
        this.pushSql(' ');
        const _r2 = this.expr(right);
        if (_r2.isErr()) return { $jump: 'return', $value: Result.Err(_r2.unwrapErr()) };
        _r2.drop();
      },
      And: (v) => {
        const left = v._0;
        const right = v._1;
        const _r3 = this.predicate(left);
        if (_r3.isErr()) return { $jump: 'return', $value: Result.Err(_r3.unwrapErr()) };
        _r3.drop();
        this.pushSql(' AND ');
        const _r4 = this.predicate(right);
        if (_r4.isErr()) return { $jump: 'return', $value: Result.Err(_r4.unwrapErr()) };
        _r4.drop();
      },
      Or: (v) => {
        const left = v._0;
        const right = v._1;
        this.pushSql('(');
        const _r5 = this.predicate(left);
        if (_r5.isErr()) return { $jump: 'return', $value: Result.Err(_r5.unwrapErr()) };
        _r5.drop();
        this.pushSql(' OR ');
        const _r6 = this.predicate(right);
        if (_r6.isErr()) return { $jump: 'return', $value: Result.Err(_r6.unwrapErr()) };
        _r6.drop();
        this.pushSql(')');
      },
      Not: (v) => {
        const pred = v._0;
        this.pushSql('NOT (');
        const _r7 = this.predicate(pred);
        if (_r7.isErr()) return { $jump: 'return', $value: Result.Err(_r7.unwrapErr()) };
        _r7.drop();
        this.pushSql(')');
      },
      IsNull: (v) => {
        const expr = v._0;
        const _r8 = this.expr(expr);
        if (_r8.isErr()) return { $jump: 'return', $value: Result.Err(_r8.unwrapErr()) };
        _r8.drop();
        this.pushSql(' IS NULL');
      },
      True: () => {
        this.pushSql('1=1');
      },
      False: () => {
        this.pushSql('1=0');
      },
      Placeholder: () => {
        return { $jump: 'return', $value: Result.Err(new SqlGenerationError('PlaceholderFound', {})) };
      },
    });
    if ((_m9 as any)?.$jump === 'return') return (_m9 as any).$value;
    return Result.Ok([]);
  }

  selection(selection: Selection): Result<void, SqlGenerationError> {
    const _r0 = this.predicate(selection.predicate);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    {
      const _v = selection.orderBy;
      if (_v != null) {
        const orderByItems = _v;
        this.pushSql(' ORDER BY ');
        for (const [i, orderBy] of [...orderByItems].entries()) {
          if (i > 0) {
            this.pushSql(', ');
          }
          const _r1 = this.orderByItem(orderBy);
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          _r1.drop();
        }
      }
    }
    {
      const _v1 = selection.limit;
      if (_v1 != null) {
        const limit = _v1;
        this.pushSql(` LIMIT ${limit}`);
      }
    }
    return Result.Ok([]);
  }

  orderByItem(orderBy: OrderByItem): Result<void, SqlGenerationError> {
    if (orderBy.path.isSimple()) {
      const escaped = orderBy.path.first().replace('"', '""');
      this.pushSql(`"${escaped}"`);
    } else {
      const first = orderBy.path.first().replace('"', '""');
      this.pushSql(`"${first}"`);
      for (const step of [...orderBy.path.steps].slice(1)) {
        const escaped = step.replace('\'', '\'\'');
        this.pushSql(`->'${escaped}'`);
      }
    }
    orderBy.direction.match({
      Asc: () => {
        this.pushSql(' ASC');
      },
      Desc: () => {
        this.pushSql(' DESC');
      },
    });
    return Result.Ok([]);
  }

  static default(): SqlBuilder {
    return SqlBuilder.new();
  }
}

export type SqlGenerationErrorV = {
  PlaceholderFound: {};
  UnsupportedExpression: { _0: string };
  UnsupportedOperator: { _0: string };
};

export class SqlGenerationError extends Enum<SqlGenerationErrorV> {

  clone(): SqlGenerationError {
    return new SqlGenerationError(this.type, { ...this.value });
  }

  debug(): string {
    return this.match({
      PlaceholderFound: () => 'PlaceholderFound',
      UnsupportedExpression: (v) => `UnsupportedExpression(${JSON.stringify(v._0)})`,
      UnsupportedOperator: (v) => `UnsupportedOperator(${JSON.stringify(v._0)})`,
    });
  }

  override toString(): string {
    return this.match({
      PlaceholderFound: () => 'Placeholder found in predicate - placeholders should be replaced before predicate processing',
      UnsupportedExpression: (v) => `Unsupported expression type: ${v._0}`,
      UnsupportedOperator: (v) => `Unsupported operator: ${v._0}`,
    });
  }
}

export function splitPredicateForSqlite(predicate: Predicate): SplitPredicate {
  const [sqlPred, remainingPred] = splitPredicateRecursive(predicate);
  return new SplitPredicate(sqlPred, remainingPred);
}

function splitPredicateRecursive(predicate: Predicate): [Predicate, Predicate] {
  return predicate.match({
    Comparison: (v) => {
      const left = v.left;
      const right = v.right;
      if (canPushdownComparison(left, right)) {
        return [predicate.clone(), new Predicate('True', {})];
      } else {
        return [new Predicate('True', {}), predicate.clone()];
      }
    },
    And: (v) => {
      const left = v._0;
      const right = v._1;
      const [leftSql, leftRemaining] = splitPredicateRecursive(left);
      const [rightSql, rightRemaining] = splitPredicateRecursive(right);
      const sqlPred = (() => {
        const _v1 = [leftSql, rightSql];
        if ((_v1[0].is('True')) && (_v1[1].is('True'))) {
          return new Predicate('True', {});
        } else if ((_v1[0].is('True'))) {
          return rightSql;
        } else if ((_v1[1].is('True'))) {
          return leftSql;
        } else {
          return new Predicate('And', { _0: leftSql, _1: rightSql });
        }
      })();
      const remainingPred = (() => {
        const _v3 = [leftRemaining, rightRemaining];
        if ((_v3[0].is('True')) && (_v3[1].is('True'))) {
          return new Predicate('True', {});
        } else if ((_v3[0].is('True'))) {
          return rightRemaining;
        } else if ((_v3[1].is('True'))) {
          return leftRemaining;
        } else {
          return new Predicate('And', { _0: leftRemaining, _1: rightRemaining });
        }
      })();
      return [sqlPred, remainingPred];
    },
    Or: (v) => {
      const left = v._0;
      const right = v._1;
      const [leftSql, leftRemaining] = splitPredicateRecursive(left);
      const [rightSql, rightRemaining] = splitPredicateRecursive(right);
      if (leftRemaining.is('True') && rightRemaining.is('True')) {
        return [predicate.clone(), new Predicate('True', {})];
      } else {
        const sqlPred = (() => {
          const _v5 = [leftSql, rightSql];
          if ((_v5[0].is('True')) && (_v5[1].is('True'))) {
            return new Predicate('True', {});
          } else if ((_v5[0].is('True'))) {
            return rightSql;
          } else if ((_v5[1].is('True'))) {
            return leftSql;
          } else {
            return new Predicate('Or', { _0: leftSql, _1: rightSql });
          }
        })();
        return [sqlPred, predicate.clone()];
      }
    },
    Not: (v) => {
      const inner = v._0;
      const [innerSql, innerRemaining] = splitPredicateRecursive(inner);
      if (innerRemaining.is('True')) {
        return [new Predicate('Not', { _0: innerSql }), new Predicate('True', {})];
      } else {
        return [new Predicate('True', {}), predicate.clone()];
      }
    },
    IsNull: (v) => {
      const expr = v._0;
      if (canPushdownExpr(expr)) {
        return [predicate.clone(), new Predicate('True', {})];
      } else {
        return [new Predicate('True', {}), predicate.clone()];
      }
    },
    True: () => [new Predicate('True', {}), new Predicate('True', {})] as any,
    False: () => [new Predicate('False', {}), new Predicate('True', {})] as any,
    Placeholder: () => [new Predicate('True', {}), predicate.clone()] as any,
  });
}

function canPushdownComparison(left: Expr, right: Expr): boolean {
  return canPushdownExpr(left) && canPushdownExpr(right);
}

function canPushdownExpr(expr: Expr): boolean {
  return expr.match({
    Literal: (v) => true,
    Path: (v) => {
      const path = v._0;
      return !(path.steps.length === 0);
    },
    ExprList: (v) => {
      const exprs = v._0;
      return [...exprs].every(canPushdownExpr) as any;
    },
    Predicate: (v) => false,
    InfixExpr: () => false,
    Placeholder: () => false,
  });
}

function comparisonOpToSql(op: ComparisonOperator): Result<string, SqlGenerationError> {
  const _m0 = (() => {
    return op.match<any>({
      Equal: () => '=',
      NotEqual: () => '<>',
      GreaterThan: () => '>',
      GreaterThanOrEqual: () => '>=',
      LessThan: () => '<',
      LessThanOrEqual: () => '<=',
      In: () => 'IN',
      Between: () => {
        return { $jump: 'return', $value: Result.Err(new SqlGenerationError('UnsupportedOperator', { _0: 'BETWEEN operator is not yet supported' })) };
      },
    });
  })();
  if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
  return Result.Ok((_m0 as any));
}

export function SqliteError_fromSqlGenerationError(err: SqlGenerationError): SqliteError {
  try {
    return new SqliteError('SqlGeneration', { _0: err.toString() });
  } finally {
    err.drop();
  }
}

