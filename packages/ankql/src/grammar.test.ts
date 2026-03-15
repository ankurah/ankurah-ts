// MIRRORS: ankurah/ankql/src/grammar.rs
// Tests the hand-written parser output for cases that the pest grammar tests covered.
// Divergence: Rust tests use parses_to! macro on pest token stream;
// TS tests verify the parsed AST from the recursive descent parser [E6].

import { describe, test, expect } from 'bun:test';
import { parseSelection } from './parser.ts';
import { PathExpr } from './ast.ts';

// ── Helpers ──

/** Assert a predicate is a Comparison with the given path/op/path or path/op/literal shape. */
function assertComparisonPaths(
  pred: { type: string; value: any },
  leftPath: string,
  op: string,
  rightPath: string,
): void {
  expect(pred.type).toBe('Comparison');
  const v = pred.value;
  expect(v.left.is('Path')).toBe(true);
  expect(v.left.value.path.toString()).toBe(leftPath);
  expect(v.operator.type).toBe(op);
  expect(v.right.is('Path')).toBe(true);
  expect(v.right.value.path.toString()).toBe(rightPath);
}

function assertComparisonLiteral(
  pred: { type: string; value: any },
  pathName: string,
  op: string,
  litType: string,
  litValue: unknown,
): void {
  expect(pred.type).toBe('Comparison');
  const v = pred.value;
  expect(v.left.is('Path')).toBe(true);
  expect(v.left.value.path.toString()).toBe(pathName);
  expect(v.operator.type).toBe(op);
  expect(v.right.is('Literal')).toBe(true);
  expect(v.right.value.literal.type).toBe(litType);
  expect(v.right.value.literal.value.value).toBe(litValue);
}

function assertOrderByItem(
  item: { path: PathExpr; direction: { type: string } },
  pathName: string,
  dir: string,
): void {
  expect(item.path.toString()).toBe(pathName);
  expect(item.direction.type).toBe(dir);
}

// ── Tests ──

describe('grammar (parser output equivalence)', () => {
  // Rust: fn test_literal_comparison()
  test('literal comparison: a=1', () => {
    const selection = parseSelection('a=1');
    assertComparisonLiteral(selection.predicate, 'a', 'Equal', 'I32', 1);
  });

  // Rust: fn test_path_comparison()
  test('path comparison: a.foo = b.foo', () => {
    const selection = parseSelection('a.foo = b.foo');
    assertComparisonPaths(selection.predicate, 'a.foo', 'Equal', 'b.foo');
  });

  // Rust: fn test_boolean_expression()
  test('boolean expression: a.foo = b.foo AND a.bar > 1 OR b.bar > 1', () => {
    // AND binds tighter than OR: OR(AND(a.foo = b.foo, a.bar > 1), b.bar > 1)
    const selection = parseSelection('a.foo = b.foo AND a.bar > 1 OR b.bar > 1');
    expect(selection.predicate.is('Or')).toBe(true);
    if (selection.predicate.is('Or')) {
      expect(selection.predicate.value.left.is('And')).toBe(true);
      expect(selection.predicate.value.right.is('Comparison')).toBe(true);
    }
  });

  // Rust: fn test_boolean_expression_parenthetical()
  test('parenthetical: (a.foo = b.foo AND a.bar > 1) OR b.bar > 1', () => {
    const selection = parseSelection('(a.foo = b.foo AND a.bar > 1) OR b.bar > 1');
    expect(selection.predicate.is('Or')).toBe(true);
    if (selection.predicate.is('Or')) {
      const left = selection.predicate.value.left;
      expect(left.is('And')).toBe(true);
      if (left.is('And')) {
        expect(left.value.left.is('Comparison')).toBe(true);
        expect(left.value.right.is('Comparison')).toBe(true);
      }
    }
  });

  // Rust: fn test_order_by_clause_basic()
  test('ORDER BY basic', () => {
    const selection = parseSelection('true ORDER BY name');
    expect(selection.predicate.is('True')).toBe(true);
    expect(selection.orderBy).not.toBeNull();
    expect(selection.orderBy!.length).toBe(1);
    assertOrderByItem(selection.orderBy![0], 'name', 'Asc');
  });

  // Rust: fn test_order_by_clause_with_direction()
  test('ORDER BY with direction', () => {
    const selection = parseSelection('true ORDER BY name DESC');
    expect(selection.orderBy).not.toBeNull();
    expect(selection.orderBy!.length).toBe(1);
    assertOrderByItem(selection.orderBy![0], 'name', 'Desc');
  });

  // Rust: fn test_limit_clause()
  test('LIMIT clause', () => {
    const selection = parseSelection('true LIMIT 10');
    expect(selection.predicate.is('True')).toBe(true);
    expect(selection.limit).toBe(10);
  });

  // Rust: fn test_order_by_and_limit()
  test('ORDER BY and LIMIT', () => {
    const selection = parseSelection("status = 'active' ORDER BY name ASC LIMIT 5");
    expect(selection.predicate.is('Comparison')).toBe(true);
    expect(selection.orderBy).not.toBeNull();
    expect(selection.orderBy!.length).toBe(1);
    assertOrderByItem(selection.orderBy![0], 'name', 'Asc');
    expect(selection.limit).toBe(5);
  });

  // Rust: fn test_order_by_multiple_items()
  test('ORDER BY multiple items', () => {
    const selection = parseSelection('true ORDER BY name ASC, created_at DESC,id');
    expect(selection.orderBy).not.toBeNull();
    expect(selection.orderBy!.length).toBe(3);
    assertOrderByItem(selection.orderBy![0], 'name', 'Asc');
    assertOrderByItem(selection.orderBy![1], 'created_at', 'Desc');
    assertOrderByItem(selection.orderBy![2], 'id', 'Asc');
  });

  // Rust: fn test_pathological_cases()
  test('pathological cases: keywords as identifiers', () => {
    // "limit" as column name
    const s1 = parseSelection('limit = 1');
    expect(s1.predicate.is('Comparison')).toBe(true);
    if (s1.predicate.is('Comparison')) {
      expect(s1.predicate.value.left.is('Path')).toBe(true);
      if (s1.predicate.value.left.is('Path')) {
        expect(s1.predicate.value.left.value.path.toString()).toBe('limit');
      }
    }

    // "order" as column name + ORDER BY
    const s2 = parseSelection('order = 1 ORDER BY name');
    expect(s2.predicate.is('Comparison')).toBe(true);
    if (s2.predicate.is('Comparison')) {
      expect(s2.predicate.value.left.is('Path')).toBe(true);
      if (s2.predicate.value.left.is('Path')) {
        expect(s2.predicate.value.left.value.path.toString()).toBe('order');
      }
    }
    expect(s2.orderBy).not.toBeNull();
    expect(s2.orderBy!.length).toBe(1);
    assertOrderByItem(s2.orderBy![0], 'name', 'Asc');
  });

  test('raw parsing: various inputs parse without error', () => {
    const testCases = [
      'true',
      'true or false',
      'true and false',
      'true LIMIT 10',
      "status = 'active'",
      "status = 'active' LIMIT 5",
      'limit = 1',
      'limit = 1 LIMIT 10',
      'foo = 1 order by name',
      'true ORDER BY name ASC',
      'true ORDER BY name DESC',
      'true ORDER BY name LIMIT 10',
      'order = 1',
      'by = 2',
      'order = 1 ORDER BY name',
    ];

    for (const input of testCases) {
      expect(() => parseSelection(input)).not.toThrow();
    }
  });
});
