// MIRRORS: ankurah/ankql/src/selection/sql.rs
import { decodeUtf8Lossy, Result, BorrowMut, checkedAdd } from '@ankurah/base';
import { ComparisonOperator, Expr, Predicate } from '../ast';
import { SqlGenerationError } from '../error';

function generateExprSql(expr: Expr, placeholderCount: BorrowMut<number | null>, foundPlaceholders: BorrowMut<number>, buffer: BorrowMut<string>): Result<void, SqlGenerationError> {
  const _m1 = expr.match<any>({
    Placeholder: () => {
      foundPlaceholders.value = checkedAdd(foundPlaceholders.value, 1, 'usize');
      {
        const _v = placeholderCount.value;
        if (_v != null) {
          const expected = _v;
          if (foundPlaceholders.value > expected) {
            return { $jump: 'return', $value: Result.Err(new SqlGenerationError('PlaceholderCountMismatch', { expected: expected, found: foundPlaceholders.value })) };
          }
        }
      }
      buffer.value += '?';
    },
    Literal: (v) => {
      const lit = v._0;
      return lit.match<any>({
        I16: (v) => {
          const i = v._0;
          buffer.value += i.toString();
        },
        I32: (v) => {
          const i = v._0;
          buffer.value += i.toString();
        },
        I64: (v) => {
          const i = v._0;
          buffer.value += i.toString();
        },
        F64: (v) => {
          const f = v._0;
          buffer.value += f.toString();
        },
        Bool: (v) => {
          const b = v._0;
          buffer.value += (b ? 'true' : 'false');
        },
        String: (v) => {
          const s = v._0;
          buffer.value += '\'';
          for (const c of [...s]) {
            if (c === '\'') {
              buffer.value += '\'\''
            } else if (c === '\u{0}') {
              continue;
            } else {
              buffer.value += c
            }
          }
          buffer.value += '\'';
        },
        EntityId: (v) => {
          const ulid = v._0;
          buffer.value += '\'';
          buffer.value += generalPurpose.URL_SAFE_NO_PAD.encode(ulid.toBytes());
          buffer.value += '\'';
        },
        Object: (v) => {
          const bytes = v._0;
          buffer.value += '\'';
          buffer.value += decodeUtf8Lossy(bytes);
          buffer.value += '\'';
        },
        Binary: (v) => {
          const bytes = v._0;
          buffer.value += '\'';
          buffer.value += decodeUtf8Lossy(bytes);
          buffer.value += '\'';
        },
        Json: (v) => {
          const value = v._0;
          buffer.value += '\'';
          buffer.value += value.toString();
          buffer.value += '\'';
        },
      });
    },
    Path: (v) => {
      const path = v._0;
      for (const [i, step] of [...path.steps].entries()) {
        if (i > 0) {
          buffer.value += '.';
        }
        buffer.value += '"';
        buffer.value += step;
        buffer.value += '"';
      }
    },
    ExprList: (v) => {
      const exprs = v._0;
      buffer.value += '(';
      for (const [i, expr] of [...exprs].entries()) {
        if (i > 0) {
          buffer.value += ', ';
        }
        const _m0 = expr.match<any>({
          Placeholder: () => {
            foundPlaceholders.value = checkedAdd(foundPlaceholders.value, 1, 'usize');
            {
              const _v1 = placeholderCount.value;
              if (_v1 != null) {
                const expected = _v1;
                if (foundPlaceholders.value > expected) {
                  return { $jump: 'return', $value: Result.Err(new SqlGenerationError('PlaceholderCountMismatch', { expected: expected, found: foundPlaceholders.value })) };
                }
              }
            }
            buffer.value += '?';
          },
          Literal: (v) => {
            const lit = v._0;
            return lit.match<any>({
              I16: (v) => {
                const i = v._0;
                buffer.value += i.toString();
              },
              I32: (v) => {
                const i = v._0;
                buffer.value += i.toString();
              },
              I64: (v) => {
                const i = v._0;
                buffer.value += i.toString();
              },
              F64: (v) => {
                const f = v._0;
                buffer.value += f.toString();
              },
              String: (v) => {
                const s = v._0;
                buffer.value += '\'';
                for (const c of [...s]) {
                  if (c === '\'') {
                    buffer.value += '\'\''
                  } else if (c === '\u{0}') {
                    continue;
                  } else {
                    buffer.value += c
                  }
                }
                buffer.value += '\'';
              },
              Bool: (v) => {
                const b = v._0;
                buffer.value += (b ? 'true' : 'false');
              },
              EntityId: (v) => {
                const ulid = v._0;
                buffer.value += '\'';
                buffer.value += generalPurpose.URL_SAFE_NO_PAD.encode(ulid.toBytes());
                buffer.value += '\'';
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
                buffer.value += '\'';
                buffer.value += value.toString();
                buffer.value += '\'';
              },
            });
          },
          Path: () => {
            return { $jump: 'return', $value: Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal expressions and placeholders are supported in IN lists' })) };
          },
          Predicate: () => {
            return { $jump: 'return', $value: Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal expressions and placeholders are supported in IN lists' })) };
          },
          InfixExpr: () => {
            return { $jump: 'return', $value: Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal expressions and placeholders are supported in IN lists' })) };
          },
          ExprList: () => {
            return { $jump: 'return', $value: Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal expressions and placeholders are supported in IN lists' })) };
          },
        });
        if ((_m0 as any)?.$jump === 'return') return _m0;
      }
      buffer.value += ')';
    },
    Predicate: () => {
      return { $jump: 'return', $value: Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal, identifier, and list expressions are supported' })) };
    },
    InfixExpr: () => {
      return { $jump: 'return', $value: Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Only literal, identifier, and list expressions are supported' })) };
    },
  });
  if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
  return Result.Ok([]);
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

export function generateSelectionSql(predicate: Predicate, expectedPlaceholders: number | null): Result<string, SqlGenerationError> {
  const placeholderCount = new BorrowMut(expectedPlaceholders);
  const foundPlaceholders = new BorrowMut(0);
  const buffer = new BorrowMut('');
  const _r0 = generateSelectionSqlInner(predicate, placeholderCount, foundPlaceholders, buffer);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  _r0.drop();
  {
    const _v = expectedPlaceholders;
    if (_v != null) {
      const expected = _v;
      if (foundPlaceholders.value !== expected) {
        return Result.Err(new SqlGenerationError('PlaceholderCountMismatch', { expected: expected, found: foundPlaceholders.value }));
      }
    }
  }
  return Result.Ok(buffer.value);
}

function generateSelectionSqlInner(predicate: Predicate, placeholderCount: BorrowMut<number | null>, foundPlaceholders: BorrowMut<number>, buffer: BorrowMut<string>): Result<void, SqlGenerationError> {
  const _m9 = predicate.match<any>({
    Comparison: (v) => {
      const left = v.left;
      const operator = v.operator;
      const right = v.right;
      const _r0 = generateExprSql(left, placeholderCount, foundPlaceholders, buffer);
      if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
      _r0.drop();
      buffer.value += ' ';
      const _r1 = comparisonOpToSql(operator);
      if (_r1.isErr()) return { $jump: 'return', $value: Result.Err(_r1.unwrapErr()) };
      buffer.value += _r1.unwrap();
      buffer.value += ' ';
      const _r2 = generateExprSql(right, placeholderCount, foundPlaceholders, buffer);
      if (_r2.isErr()) return { $jump: 'return', $value: Result.Err(_r2.unwrapErr()) };
      _r2.drop();
    },
    And: (v) => {
      const left = v._0;
      const right = v._1;
      const _r3 = generateSelectionSqlInner(left, placeholderCount, foundPlaceholders, buffer);
      if (_r3.isErr()) return { $jump: 'return', $value: Result.Err(_r3.unwrapErr()) };
      _r3.drop();
      buffer.value += ' AND ';
      const _r4 = generateSelectionSqlInner(right, placeholderCount, foundPlaceholders, buffer);
      if (_r4.isErr()) return { $jump: 'return', $value: Result.Err(_r4.unwrapErr()) };
      _r4.drop();
    },
    Or: (v) => {
      const left = v._0;
      const right = v._1;
      buffer.value += '(';
      const _r5 = generateSelectionSqlInner(left, placeholderCount, foundPlaceholders, buffer);
      if (_r5.isErr()) return { $jump: 'return', $value: Result.Err(_r5.unwrapErr()) };
      _r5.drop();
      buffer.value += ' OR ';
      const _r6 = generateSelectionSqlInner(right, placeholderCount, foundPlaceholders, buffer);
      if (_r6.isErr()) return { $jump: 'return', $value: Result.Err(_r6.unwrapErr()) };
      _r6.drop();
      buffer.value += ')';
    },
    Not: (v) => {
      const pred = v._0;
      buffer.value += 'NOT (';
      const _r7 = generateSelectionSqlInner(pred, placeholderCount, foundPlaceholders, buffer);
      if (_r7.isErr()) return { $jump: 'return', $value: Result.Err(_r7.unwrapErr()) };
      _r7.drop();
      buffer.value += ')';
    },
    IsNull: (v) => {
      const expr = v._0;
      const _r8 = generateExprSql(expr, placeholderCount, foundPlaceholders, buffer);
      if (_r8.isErr()) return { $jump: 'return', $value: Result.Err(_r8.unwrapErr()) };
      _r8.drop();
      buffer.value += ' IS NULL';
    },
    True: () => {
      buffer.value += 'TRUE';
    },
    False: () => {
      buffer.value += 'FALSE';
    },
    Placeholder: () => {
      return { $jump: 'return', $value: Result.Err(new SqlGenerationError('InvalidExpression', { _0: 'Placeholder must be transformed before SQL generation' })) };
    },
  });
  if ((_m9 as any)?.$jump === 'return') return (_m9 as any).$value;
  return Result.Ok([]);
}

