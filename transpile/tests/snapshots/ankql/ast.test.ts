// MIRRORS: ankurah/ankql/src/ast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { ComparisonOperator, Expr, Literal, PathExpr, Predicate } from './ast';
import { Result } from '@ankurah/base';
import { ParseError } from './error';
import { parseSelection } from './parser';
import { generateSelectionSql } from './selection/sql';

describe('ast unit tests', () => {
  function nullifyColumns(input: string, nullColumns: string[]): Result<string, ParseError> {
    const _r0 = parseSelection(input);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const selection = _r0.unwrap();
    const result = selection.predicate.assumeNull([...nullColumns].map((s) => s));
    return generateSelectionSql(result, null).mapErr((_) => new ParseError('InvalidPredicate', { _0: 'SQL generation failed' }));
  }

  test('test_single_comparison_null_handling', () => {
    expect(nullifyColumns('status = \'active\'', ['status'])).toEqual('FALSE');
    expect(nullifyColumns('age > 30', ['age'])).toEqual('FALSE');
    expect(nullifyColumns('count >= 100', ['count'])).toEqual('FALSE');
    expect(nullifyColumns('name < \'Z\'', ['name'])).toEqual('FALSE');
    expect(nullifyColumns('score <= 90', ['score'])).toEqual('FALSE');
    expect(nullifyColumns('status IS NULL', ['status'])).toEqual('TRUE');
    expect(nullifyColumns('role = \'admin\'', ['other'])).toEqual('"role" = \'admin\'');
  });

  test('nested_predicate_null_handling', () => {
    const input = 'alpha = 1 AND (beta = 2 OR charlie = 3)';
    expect(nullifyColumns(input, ['charlie'])).toEqual('"alpha" = 1 AND "beta" = 2');
    expect(nullifyColumns(input, ['beta', 'charlie'])).toEqual('FALSE');
    expect(nullifyColumns(input, ['alpha'])).toEqual('FALSE');
    expect(nullifyColumns(input, ['other'])).toEqual('"alpha" = 1 AND ("beta" = 2 OR "charlie" = 3)');
  });

  test('test_populate_single_placeholder', () => {
    const selection = parseSelection('name = ?');
    const populated = selection.predicate.populate(['Alice']);
    const expected = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('name') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'Alice' }) }) });
    try {
      expect(populated).toEqual(expected);
    } finally {
      expected.drop();
    }
  });

  test('test_populate_multiple_placeholders', () => {
    const selection = parseSelection('age > ? AND name = ?');
    const values = [(25n), 'Bob'];
    const populated = selection.predicate.populate(values);
    const expected = new Predicate('And', { _0: new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('age') }), operator: new ComparisonOperator('GreaterThan', {}), right: new Expr('Literal', { _0: new Literal('I64', { _0: 25n }) }) }), _1: new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('name') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'Bob' }) }) }) });
    try {
      expect(populated).toEqual(expected);
    } finally {
      expected.drop();
    }
  });

  test('test_populate_in_clause', () => {
    const selection = parseSelection('status IN (?, ?, ?)');
    const populated = selection.predicate.populate(['active', 'pending', 'review']);
    const expected = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('status') }), operator: new ComparisonOperator('In', {}), right: new Expr('ExprList', { _0: [new Expr('Literal', { _0: new Literal('String', { _0: 'active' }) }), new Expr('Literal', { _0: new Literal('String', { _0: 'pending' }) }), new Expr('Literal', { _0: new Literal('String', { _0: 'review' }) })] }) });
    try {
      expect(populated).toEqual(expected);
    } finally {
      expected.drop();
    }
  });

  test('test_populate_mixed_types', () => {
    const selection = parseSelection('active = ? AND score > ? AND name = ?');
    const values = [true, (95.5), 'Charlie'];
    const populated = selection.predicate.populate(values);
    {
      const _v4 = populated;
      if (_v4.is('And')) {
        const { _0: left, _1: right } = _v4.value;
        {
          const _v2 = left;
          if (_v2.is('And')) {
            const { _0: innerLeft, _1: innerRight } = _v2.value;
            {
              const _v = innerLeft;
              if (_v.is('Comparison')) {
                const { right: val } = _v.value;
                const _t0 = new Expr('Literal', { _0: new Literal('Bool', { _0: true }) });
                try {
                  expect(val).toEqual(_t0);
                } finally {
                  _t0.drop();
                }
              }
            }
            {
              const _v1 = innerRight;
              if (_v1.is('Comparison')) {
                const { right: val } = _v1.value;
                const _t1 = new Expr('Literal', { _0: new Literal('F64', { _0: 95.5 }) });
                try {
                  expect(val).toEqual(_t1);
                } finally {
                  _t1.drop();
                }
              }
            }
          }
        }
        {
          const _v3 = right;
          if (_v3.is('Comparison')) {
            const { right: val } = _v3.value;
            const _t2 = new Expr('Literal', { _0: new Literal('String', { _0: 'Charlie' }) });
            try {
              expect(val).toEqual(_t2);
            } finally {
              _t2.drop();
            }
          }
        }
      }
    }
  });

  test('test_populate_too_few_values', () => {
    const selection = parseSelection('name = ? AND age = ?');
    const result = selection.predicate.populate(['Alice']);
    if (!(result.isErr())) throw new Error('assertion failed');
    if (!(result.unwrapErr().toString().includes('Not enough values'))) throw new Error('assertion failed');
  });

  test('test_populate_too_many_values', () => {
    const selection = parseSelection('name = ?');
    const result = selection.predicate.populate(['Alice', 'Bob']);
    if (!(result.isErr())) throw new Error('assertion failed');
    if (!(result.unwrapErr().toString().includes('Too many values'))) throw new Error('assertion failed');
  });

  test('test_populate_no_placeholders', () => {
    const selection = parseSelection('name = \'Alice\'');
    const populated = selection.clone().predicate.populate([]);
    expect(populated).toEqual(selection.predicate);
  });

});
