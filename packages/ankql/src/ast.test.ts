// MIRRORS: ankurah/ankql/src/ast.rs

import { describe, test, expect } from 'bun:test';
import { parseSelection } from './parser.ts';
import { generateSelectionSql } from './selection/sql.ts';
import {
  PathExpr,
  assumeNull,
  populatePredicate,
  exprFromString,
  exprFromI64,
  exprFromF64,
  exprFromBool,
} from './ast.ts';
import type { Predicate, Expr, Literal } from './ast.ts';

// Helper: parse input, null-ify columns, generate SQL
function nullifyColumns(input: string, nullColumns: string[]): string {
  const selection = parseSelection(input);
  const result = assumeNull(selection.predicate, nullColumns);
  return generateSelectionSql(result);
}

describe('assume_null', () => {
  test('single comparison null handling', () => {
    expect(nullifyColumns("status = 'active'", ['status'])).toBe('FALSE');
    expect(nullifyColumns('age > 30', ['age'])).toBe('FALSE');
    expect(nullifyColumns('count >= 100', ['count'])).toBe('FALSE');
    expect(nullifyColumns("name < 'Z'", ['name'])).toBe('FALSE');
    expect(nullifyColumns('score <= 90', ['score'])).toBe('FALSE');
    expect(nullifyColumns('status IS NULL', ['status'])).toBe('TRUE');
    expect(nullifyColumns("role = 'admin'", ['other'])).toBe('"role" = \'admin\'');
  });

  test('nested predicate null handling', () => {
    const input = 'alpha = 1 AND (beta = 2 OR charlie = 3)';
    expect(nullifyColumns(input, ['charlie'])).toBe('"alpha" = 1 AND "beta" = 2');
    expect(nullifyColumns(input, ['beta', 'charlie'])).toBe('FALSE');
    expect(nullifyColumns(input, ['alpha'])).toBe('FALSE');
    expect(nullifyColumns(input, ['other'])).toBe('"alpha" = 1 AND ("beta" = 2 OR "charlie" = 3)');
  });
});

describe('populate', () => {
  test('single placeholder', () => {
    const selection = parseSelection('name = ?');
    const populated = populatePredicate(selection.predicate, [
      exprFromString('Alice'),
    ]);

    expect(populated).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('name') },
      operator: 'Equal',
      right: { type: 'Literal', value: { type: 'String', value: 'Alice' } },
    });
  });

  test('multiple placeholders', () => {
    const selection = parseSelection('age > ? AND name = ?');
    const populated = populatePredicate(selection.predicate, [
      exprFromI64(25n),
      exprFromString('Bob'),
    ]);

    expect(populated).toEqual({
      type: 'And',
      left: {
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('age') },
        operator: 'GreaterThan',
        right: { type: 'Literal', value: { type: 'I64', value: 25n } },
      },
      right: {
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('name') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'String', value: 'Bob' } },
      },
    });
  });

  test('IN clause placeholders', () => {
    const selection = parseSelection('status IN (?, ?, ?)');
    const populated = populatePredicate(selection.predicate, [
      exprFromString('active'),
      exprFromString('pending'),
      exprFromString('review'),
    ]);

    expect(populated).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('status') },
      operator: 'In',
      right: {
        type: 'ExprList',
        values: [
          { type: 'Literal', value: { type: 'String', value: 'active' } },
          { type: 'Literal', value: { type: 'String', value: 'pending' } },
          { type: 'Literal', value: { type: 'String', value: 'review' } },
        ],
      },
    });
  });

  test('mixed types', () => {
    const selection = parseSelection('active = ? AND score > ? AND name = ?');
    const populated = populatePredicate(selection.predicate, [
      exprFromBool(true),
      exprFromF64(95.5),
      exprFromString('Charlie'),
    ]);

    // Verify the structure
    if (populated.type === 'And') {
      if (populated.left.type === 'And') {
        // Check boolean value
        if (populated.left.left.type === 'Comparison') {
          expect(populated.left.left.right).toEqual({
            type: 'Literal',
            value: { type: 'Bool', value: true },
          });
        }
        // Check float value
        if (populated.left.right.type === 'Comparison') {
          expect(populated.left.right.right).toEqual({
            type: 'Literal',
            value: { type: 'F64', value: 95.5 },
          });
        }
      }
      // Check string value
      if (populated.right.type === 'Comparison') {
        expect(populated.right.right).toEqual({
          type: 'Literal',
          value: { type: 'String', value: 'Charlie' },
        });
      }
    }
  });

  test('too few values', () => {
    const selection = parseSelection('name = ? AND age = ?');
    expect(() =>
      populatePredicate(selection.predicate, [exprFromString('Alice')]),
    ).toThrow(/Not enough values/);
  });

  test('too many values', () => {
    const selection = parseSelection('name = ?');
    expect(() =>
      populatePredicate(selection.predicate, [
        exprFromString('Alice'),
        exprFromString('Bob'),
      ]),
    ).toThrow(/Too many values/);
  });

  test('no placeholders', () => {
    const selection = parseSelection("name = 'Alice'");
    const populated = populatePredicate(selection.predicate, []);
    expect(populated).toEqual(selection.predicate);
  });
});
