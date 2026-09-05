// MIRRORS: ankurah/ankql/src/selection/sql.rs
import { Result } from '@ankurah/base';
import { ComparisonOperator, Expr, Literal, Predicate } from '../ast';
import { SqlGenerationError } from '../error';

function generateExprSql(expr: Expr, placeholderCount: number | null, foundPlaceholders: number, buffer: string): Result<void, SqlGenerationError> {
  expr.match({
    Placeholder: () => {
      foundPlaceholders.value += 1;
      {
        const _v = placeholderCount;
        if (_v != null) {
          const expected = _v;
          if (foundPlaceholders > expected) {
            return Result.Err(new SqlGenerationError('PlaceholderCountMismatch', { expected: expected, found: foundPlaceholders }));
          }
        }
      }
      buffer += '?';
    },
    Literal: (v) => {
      const lit = v._0;
      lit.match({
        I16: (v) => {
          const i = v._0;
          buffer += i.toString();
        },
        I32: (v) => {
          const i = v._0;
          buffer += i.toString();
        },
        I64: (v) => {
          const i = v._0;
          buffer += i.toString();
        },
        F64: (v) => {
          const f = v._0;
          buffer += f.toString();
        },
        Bool: (v) => {
          const b = v._0;
          buffer += b ? 'true' : 'false';
        },
        String: (v) => {
          const s = v._0;
          buffer += '\'';
          for (const c of [...s]) {
            if (c === '\'') {
              buffer += '\'\'';
            } else if (c === '\0') {
              {
                continue;
              }
            } else {
              buffer += c;
            }
          }
          buffer += '\'';
        },
        EntityId: (v) => {
          const ulid = v._0;
          buffer += '\'';
          buffer += generalPurpose.URL_SAFE_NO_PAD.encode(ulid.toBytes());
          buffer += '\'';
        },
        Object: (v) => {
          const bytes = v._0;
          buffer += '\'';
          buffer += String.fromUtf8Lossy(bytes);
          buffer += '\'';
        },
        Binary: (v) => {
          const bytes = v._0;
          buffer += '\'';
          buffer += String.fromUtf8Lossy(bytes);
          buffer += '\'';
        },
        Json: (v) => {
          const value = v._0;
          buffer += '\'';
          buffer += value.toString();
          buffer += '\'';
        },
      })
    },
    Path: (v) => {
      const path = v._0;
      for (const [i, step] of [...path.steps].entries()) {
        if (i > 0) {
          buffer += '.';
        }
        buffer += '"';
        buffer += step;
        buffer += '"';
      }
    },
    ExprList: (v) => {
      const exprs = v._0;
      buffer += '(';
      for (const [i, expr] of [...exprs].entries()) {
        if (i > 0) {
          buffer += ', ';
        }
        return expr.match({
          Placeholder: () => {
            foundPlaceholders.value += 1;
            {
              const _v1 = placeholderCount;
              if (_v1 != null) {
                const expected = _v1;
                if (foundPlaceholders > expected) {
                  return Result.Err(new SqlGenerationError('PlaceholderCountMismatch', { expected: expected, found: foundPlaceholders }));
                }
              }
            }
            buffer += '?';
          },
          Literal: (v) => {
            const lit = v._0;
            lit.match({
              I16: (v) => {
                const i = v._0;
                buffer += i.toString();
              },
              I32: (v) => {
                const i = v._0;
                buffer += i.toString();
              },
              I64: (v) => {
                const i = v._0;
                buffer += i.toString();
              },
              F64: (v) => {
                const f = v._0;
                buffer += f.toString();
              },
              String: (v) => {
                const s = v._0;
                buffer += '\'';
                for (const c of [...s]) {
                  if (c === '\'') {
                    buffer += '\'\'';
                  } else if (c === '\0') {
                    {
                      continue;
                    }
                  } else {
                    buffer += c;
                  }
                }
                buffer += '\'';
              },
              Bool: (v) => {
                const b = v._0;
                buffer += b ? 'true' : 'false';
              },
              EntityId: (v) => {
                const ulid = v._0;
                buffer += '\'';
                buffer += generalPurpose.URL_SAFE_NO_PAD.encode(ulid.toBytes());
                buffer += '\'';
              },
              Object: (v) => {
                const _bytes = v._0;
                throw new Error('TODO');
              },
              Binary: (v) => {
                const _bytes = v._0;
                throw new Error('TODO');
              },
              Json: (v) => {
                const value = v._0;
                buffer += '\'';
                buffer += value.toString();
                buffer += '\'';
              },
            })
          },
          Path: () => {
            return Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal expressions and placeholders are supported in IN lists' }));
          },
          Predicate: () => {
            return Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal expressions and placeholders are supported in IN lists' }));
          },
          InfixExpr: () => {
            return Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal expressions and placeholders are supported in IN lists' }));
          },
          ExprList: () => {
            return Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal expressions and placeholders are supported in IN lists' }));
          },
        });
      }
      buffer += ')';
    },
    Predicate: () => {
      return Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal, identifier, and list expressions are supported' }))
    },
    InfixExpr: () => {
      return Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal, identifier, and list expressions are supported' }))
    },
  })
  return Result.Ok([]);
}

function comparisonOpToSql(op: ComparisonOperator): Result<string, SqlGenerationError> {
  return Result.Ok(op.match({
    Equal: () => '=',
    NotEqual: () => '<>',
    GreaterThan: () => '>',
    GreaterThanOrEqual: () => '>=',
    LessThan: () => '<',
    LessThanOrEqual: () => '<=',
    In: () => 'IN',
    Between: () => {
      return Result.Err(new SqlGenerationError('UnsupportedOperator', { _0: 'BETWEEN operator is not yet supported' }))
    },
  }));
}

export function generateSelectionSql(predicate: Predicate, expectedPlaceholders: number | null): Result<string, SqlGenerationError> {
  let placeholderCount = expectedPlaceholders;
  let foundPlaceholders = 0;
  let buffer = '';
  const _r0 = generateSelectionSqlInner(predicate, placeholderCount, foundPlaceholders, buffer);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  _r0.drop();
  {
    const _v = expectedPlaceholders;
    if (_v != null) {
      const expected = _v;
      if (foundPlaceholders !== expected) {
        return Result.Err(new SqlGenerationError('PlaceholderCountMismatch', { expected: expected, found: foundPlaceholders }));
      }
    }
  }
  return Result.Ok(buffer);
}

function generateSelectionSqlInner(predicate: Predicate, placeholderCount: number | null, foundPlaceholders: number, buffer: string): Result<void, SqlGenerationError> {
  predicate.match({
    Comparison: (v) => {
      const left = v.left;
      const operator = v.operator;
      const right = v.right;
      const _r0 = generateExprSql(left, placeholderCount, foundPlaceholders, buffer);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      _r0.drop();
      buffer += ' ';
      const _r1 = comparisonOpToSql(operator);
      if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
      buffer += _r1.unwrap();
      buffer += ' ';
      const _r2 = generateExprSql(right, placeholderCount, foundPlaceholders, buffer);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      _r2.drop();
    },
    And: (v) => {
      const left = v._0;
      const right = v._1;
      const _r3 = generateSelectionSqlInner(left, placeholderCount, foundPlaceholders, buffer);
      if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
      _r3.drop();
      buffer += ' AND ';
      const _r4 = generateSelectionSqlInner(right, placeholderCount, foundPlaceholders, buffer);
      if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
      _r4.drop();
    },
    Or: (v) => {
      const left = v._0;
      const right = v._1;
      buffer += '(';
      const _r5 = generateSelectionSqlInner(left, placeholderCount, foundPlaceholders, buffer);
      if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
      _r5.drop();
      buffer += ' OR ';
      const _r6 = generateSelectionSqlInner(right, placeholderCount, foundPlaceholders, buffer);
      if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
      _r6.drop();
      buffer += ')';
    },
    Not: (v) => {
      const pred = v._0;
      buffer += 'NOT (';
      const _r7 = generateSelectionSqlInner(pred, placeholderCount, foundPlaceholders, buffer);
      if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
      _r7.drop();
      buffer += ')';
    },
    IsNull: (v) => {
      const expr = v._0;
      const _r8 = generateExprSql(expr, placeholderCount, foundPlaceholders, buffer);
      if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
      _r8.drop();
      buffer += ' IS NULL';
    },
    True: () => {
      buffer += 'TRUE';
    },
    False: () => {
      buffer += 'FALSE';
    },
    Placeholder: () => {
      return Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Placeholder must be transformed before SQL generation' }));
    },
  })
  return Result.Ok([]);
}

