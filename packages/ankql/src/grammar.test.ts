// MIRRORS: ankurah/ankql/src/grammar.rs
// Tests the hand-written parser output for cases that the pest grammar tests covered.

import { describe, test, expect } from 'bun:test';
import { parseSelection } from './parser.ts';
import { PathExpr } from './ast.ts';

describe('grammar (parser output equivalence)', () => {
  test('literal comparison: a=1', () => {
    const selection = parseSelection('a=1');
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('a') },
      operator: 'Equal',
      right: { type: 'Literal', value: { type: 'I32', value: 1 } },
    });
  });

  test('path comparison: a.foo = b.foo', () => {
    const selection = parseSelection('a.foo = b.foo');
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: new PathExpr(['a', 'foo']) },
      operator: 'Equal',
      right: { type: 'Path', value: new PathExpr(['b', 'foo']) },
    });
  });

  test('boolean expression: a.foo = b.foo AND a.bar > 1 OR b.bar > 1', () => {
    // In Rust pest, this was a flat sequence. In our recursive descent parser,
    // AND binds tighter than OR, so the AST structure is:
    //   OR(AND(a.foo = b.foo, a.bar > 1), b.bar > 1)
    const selection = parseSelection('a.foo = b.foo AND a.bar > 1 OR b.bar > 1');
    expect(selection.predicate.type).toBe('Or');
    if (selection.predicate.type === 'Or') {
      expect(selection.predicate.left.type).toBe('And');
      expect(selection.predicate.right.type).toBe('Comparison');
    }
  });

  test('parenthetical: (a.foo = b.foo AND a.bar > 1) OR b.bar > 1', () => {
    const selection = parseSelection('(a.foo = b.foo AND a.bar > 1) OR b.bar > 1');
    expect(selection.predicate.type).toBe('Or');
    if (selection.predicate.type === 'Or') {
      expect(selection.predicate.left.type).toBe('And');
      if (selection.predicate.left.type === 'And') {
        expect(selection.predicate.left.left.type).toBe('Comparison');
        expect(selection.predicate.left.right.type).toBe('Comparison');
      }
    }
  });

  test('ORDER BY basic', () => {
    const selection = parseSelection('true ORDER BY name');
    expect(selection.predicate).toEqual({ type: 'True' });
    expect(selection.orderBy).toEqual([
      { path: PathExpr.simple('name'), direction: 'Asc' },
    ]);
  });

  test('ORDER BY with direction', () => {
    const selection = parseSelection('true ORDER BY name DESC');
    expect(selection.orderBy).toEqual([
      { path: PathExpr.simple('name'), direction: 'Desc' },
    ]);
  });

  test('LIMIT clause', () => {
    const selection = parseSelection('true LIMIT 10');
    expect(selection.predicate).toEqual({ type: 'True' });
    expect(selection.limit).toBe(10);
  });

  test('ORDER BY and LIMIT', () => {
    const selection = parseSelection("status = 'active' ORDER BY name ASC LIMIT 5");
    expect(selection.predicate.type).toBe('Comparison');
    expect(selection.orderBy).toEqual([
      { path: PathExpr.simple('name'), direction: 'Asc' },
    ]);
    expect(selection.limit).toBe(5);
  });

  test('ORDER BY multiple items', () => {
    const selection = parseSelection('true ORDER BY name ASC, created_at DESC,id');
    expect(selection.orderBy).toEqual([
      { path: PathExpr.simple('name'), direction: 'Asc' },
      { path: PathExpr.simple('created_at'), direction: 'Desc' },
      { path: PathExpr.simple('id'), direction: 'Asc' },
    ]);
  });

  test('pathological cases: keywords as identifiers', () => {
    // "limit" as column name
    const s1 = parseSelection('limit = 1');
    expect(s1.predicate.type).toBe('Comparison');
    if (s1.predicate.type === 'Comparison') {
      expect(s1.predicate.left).toEqual({ type: 'Path', value: PathExpr.simple('limit') });
    }

    // "order" as column name + ORDER BY
    const s2 = parseSelection('order = 1 ORDER BY name');
    expect(s2.predicate.type).toBe('Comparison');
    if (s2.predicate.type === 'Comparison') {
      expect(s2.predicate.left).toEqual({ type: 'Path', value: PathExpr.simple('order') });
    }
    expect(s2.orderBy).toEqual([
      { path: PathExpr.simple('name'), direction: 'Asc' },
    ]);
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
