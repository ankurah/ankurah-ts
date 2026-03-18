// MIRRORS: ankurah/ankql/src/ast.rs #[cfg(test)] mod tests
import { describe, test, expect } from 'bun:test';
import { parseSelection } from './parser.ts';
import { generateSelectionSql } from './selection/sql.ts';
import {
  Predicate,
  Expr,
  Literal,
  ComparisonOperator,
  PathExpr,
  assumeNull,
  populatePredicate,
  exprFromString,
  exprFromI64,
  exprFromF64,
  exprFromBool,
} from './ast.ts';

// ── Helpers ──

// Rust: fn nullify_columns
/** Parse input, null-ify columns, generate SQL. */
function nullifyColumns(input: string, nullColumns: string[]): string {
  const selection = parseSelection(input);
  const result = assumeNull(selection.predicate, nullColumns);
  return generateSelectionSql(result);
}

/** Assert a Predicate is a Comparison with path/op/literal. */
function assertComparison(
  pred: Predicate,
  pathName: string,
  opType: ComparisonOperator['type'],
  literal: { type: Literal['type']; check: (v: any) => void },
): void {
  expect(pred.is('Comparison')).toBe(true);
  pred.match({
    Comparison: (v) => {
      expect(v.left.is('Path')).toBe(true);
      if (v.left.is('Path')) {
        expect(v.left.value.path.toString()).toBe(pathName);
      }
      expect(v.operator.type).toBe(opType);
      expect(v.right.is('Literal')).toBe(true);
      if (v.right.is('Literal')) {
        expect(v.right.value.literal.type).toBe(literal.type);
        literal.check(v.right.value.literal);
      }
    },
    And: () => { throw new Error('expected Comparison'); },
    Or: () => { throw new Error('expected Comparison'); },
    Not: () => { throw new Error('expected Comparison'); },
    IsNull: () => { throw new Error('expected Comparison'); },
    True: () => { throw new Error('expected Comparison'); },
    False: () => { throw new Error('expected Comparison'); },
    Placeholder: () => { throw new Error('expected Comparison'); },
  });
}

// ── Tests ──

// Rust: fn test_single_comparison_null_handling()
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

  // Rust: fn nested_predicate_null_handling()
  test('nested predicate null handling', () => {
    const input = 'alpha = 1 AND (beta = 2 OR charlie = 3)';
    expect(nullifyColumns(input, ['charlie'])).toBe('"alpha" = 1 AND "beta" = 2');
    expect(nullifyColumns(input, ['beta', 'charlie'])).toBe('FALSE');
    expect(nullifyColumns(input, ['alpha'])).toBe('FALSE');
    expect(nullifyColumns(input, ['other'])).toBe('"alpha" = 1 AND ("beta" = 2 OR "charlie" = 3)');
  });
});

describe('populate', () => {
  // Rust: fn test_populate_single_placeholder()
  test('single placeholder', () => {
    const selection = parseSelection('name = ?');
    const populated = populatePredicate(selection.predicate, [
      exprFromString('Alice'),
    ]);

    assertComparison(populated, 'name', 'Equal', {
      type: 'String',
      check: (lit) => expect(lit.value.value).toBe('Alice'),
    });
  });

  // Rust: fn test_populate_multiple_placeholders()
  test('multiple placeholders', () => {
    const selection = parseSelection('age > ? AND name = ?');
    const populated = populatePredicate(selection.predicate, [
      exprFromI64(25n),
      exprFromString('Bob'),
    ]);

    expect(populated.is('And')).toBe(true);
    if (populated.is('And')) {
      assertComparison(populated.value.left, 'age', 'GreaterThan', {
        type: 'I64',
        check: (lit) => expect(lit.value.value).toBe(25n),
      });
      assertComparison(populated.value.right, 'name', 'Equal', {
        type: 'String',
        check: (lit) => expect(lit.value.value).toBe('Bob'),
      });
    }
  });

  // Rust: fn test_populate_in_clause()
  test('IN clause placeholders', () => {
    const selection = parseSelection('status IN (?, ?, ?)');
    const populated = populatePredicate(selection.predicate, [
      exprFromString('active'),
      exprFromString('pending'),
      exprFromString('review'),
    ]);

    expect(populated.is('Comparison')).toBe(true);
    if (populated.is('Comparison')) {
      expect(populated.value.left.is('Path')).toBe(true);
      if (populated.value.left.is('Path')) {
        expect(populated.value.left.value.path.toString()).toBe('status');
      }
      expect(populated.value.operator.type).toBe('In');
      expect(populated.value.right.is('ExprList')).toBe(true);
      if (populated.value.right.is('ExprList')) {
        const exprs = populated.value.right.value.exprs;
        expect(exprs.length).toBe(3);
        for (const [i, expected] of (['active', 'pending', 'review'] as const).entries()) {
          expect(exprs[i].is('Literal')).toBe(true);
          if (exprs[i].is('Literal')) {
            expect(exprs[i].value.literal.type).toBe('String');
            if (exprs[i].value.literal.is('String')) {
              expect(exprs[i].value.literal.value.value).toBe(expected);
            }
          }
        }
      }
    }
  });

  // Rust: fn test_populate_mixed_types()
  test('mixed types', () => {
    const selection = parseSelection('active = ? AND score > ? AND name = ?');
    const populated = populatePredicate(selection.predicate, [
      exprFromBool(true),
      exprFromF64(95.5),
      exprFromString('Charlie'),
    ]);

    // Structure: And(And(active=true, score>95.5), name='Charlie')
    expect(populated.is('And')).toBe(true);
    if (populated.is('And')) {
      const left = populated.value.left;
      const right = populated.value.right;

      expect(left.is('And')).toBe(true);
      if (left.is('And')) {
        // Check boolean value
        assertComparison(left.value.left, 'active', 'Equal', {
          type: 'Bool',
          check: (lit) => expect(lit.value.value).toBe(true),
        });
        // Check float value
        assertComparison(left.value.right, 'score', 'GreaterThan', {
          type: 'F64',
          check: (lit) => expect(lit.value.value).toBe(95.5),
        });
      }
      // Check string value
      assertComparison(right, 'name', 'Equal', {
        type: 'String',
        check: (lit) => expect(lit.value.value).toBe('Charlie'),
      });
    }
  });

  // Rust: fn test_populate_too_few_values()
  test('too few values', () => {
    const selection = parseSelection('name = ? AND age = ?');
    expect(() =>
      populatePredicate(selection.predicate, [exprFromString('Alice')]),
    ).toThrow(/Not enough values/);
  });

  // Rust: fn test_populate_too_many_values()
  test('too many values', () => {
    const selection = parseSelection('name = ?');
    expect(() =>
      populatePredicate(selection.predicate, [
        exprFromString('Alice'),
        exprFromString('Bob'),
      ]),
    ).toThrow(/Too many values/);
  });

  // Rust: fn test_populate_no_placeholders()
  test('no placeholders', () => {
    const selection = parseSelection("name = 'Alice'");
    const populated = populatePredicate(selection.predicate, []);

    // Should have same structure as original
    expect(populated.type).toBe(selection.predicate.type);
    assertComparison(populated, 'name', 'Equal', {
      type: 'String',
      check: (lit) => expect(lit.value.value).toBe('Alice'),
    });
  });
});
