// MIRRORS: ankurah/ankql/src/parser.rs

import { describe, test, expect } from 'bun:test';
import { parseSelection } from './parser.ts';
import { PathExpr, Selection } from './ast.ts';
import type { Predicate, Expr } from './ast.ts';

describe('parser', () => {
  test('parse selection: status = active', () => {
    const selection = parseSelection("status = 'active'");
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('status') },
      operator: 'Equal',
      right: { type: 'Literal', value: { type: 'String', value: 'active' } },
    });
    expect(selection.orderBy).toBeNull();
    expect(selection.limit).toBeNull();
  });

  test('parse selection: user AND status', () => {
    const selection = parseSelection("user = 123 AND status = 'active'");
    expect(selection.predicate).toEqual({
      type: 'And',
      left: {
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('user') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'I32', value: 123 } },
      },
      right: {
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('status') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'String', value: 'active' } },
      },
    });
  });

  test('parse selection: (user OR user) AND status', () => {
    const selection = parseSelection("(user = 123 OR user = 456) AND status = 'active'");
    expect(selection.predicate).toEqual({
      type: 'And',
      left: {
        type: 'Or',
        left: {
          type: 'Comparison',
          left: { type: 'Path', value: PathExpr.simple('user') },
          operator: 'Equal',
          right: { type: 'Literal', value: { type: 'I32', value: 123 } },
        },
        right: {
          type: 'Comparison',
          left: { type: 'Path', value: PathExpr.simple('user') },
          operator: 'Equal',
          right: { type: 'Literal', value: { type: 'I32', value: 456 } },
        },
      },
      right: {
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('status') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'String', value: 'active' } },
      },
    } satisfies Predicate);
  });

  test('parse selection: status IS NULL', () => {
    const selection = parseSelection('status IS NULL');
    expect(selection.predicate).toEqual({
      type: 'IsNull',
      expr: { type: 'Path', value: PathExpr.simple('status') },
    });
  });

  test('parse selection: status IS NOT NULL', () => {
    const selection = parseSelection('status IS NOT NULL');
    expect(selection.predicate).toEqual({
      type: 'Not',
      predicate: {
        type: 'IsNull',
        expr: { type: 'Path', value: PathExpr.simple('status') },
      },
    });
  });

  test('unary NOT parenthesized', () => {
    const selection = parseSelection("NOT (status = 'active')");
    expect(selection.predicate).toEqual({
      type: 'Not',
      predicate: {
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('status') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'String', value: 'active' } },
      },
    });
  });

  test('unary NOT unparenthesized fails', () => {
    expect(() => parseSelection("NOT status = 'active'")).toThrow();
  });

  test('parse empty string', () => {
    const selection = parseSelection('');
    expect(selection.predicate).toEqual({ type: 'True' });
  });

  test('parse true literal', () => {
    const selection = parseSelection('true');
    expect(selection.predicate).toEqual({ type: 'True' });
  });

  test('parse false literal', () => {
    expect(parseSelection('false').predicate).toEqual({ type: 'False' });
  });

  test('parse selection: IN clause with strings', () => {
    const selection = parseSelection("status IN ('active', 'pending')");
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('status') },
      operator: 'In',
      right: {
        type: 'ExprList',
        values: [
          { type: 'Literal', value: { type: 'String', value: 'active' } },
          { type: 'Literal', value: { type: 'String', value: 'pending' } },
        ],
      },
    });
  });

  test('parse selection: IN clause with numbers', () => {
    const selection = parseSelection('user_id IN (1, 2, 3)');
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('user_id') },
      operator: 'In',
      right: {
        type: 'ExprList',
        values: [
          { type: 'Literal', value: { type: 'I32', value: 1 } },
          { type: 'Literal', value: { type: 'I32', value: 2 } },
          { type: 'Literal', value: { type: 'I32', value: 3 } },
        ],
      },
    });
  });

  test('comparison to true', () => {
    const selection = parseSelection('bool_field = true');
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('bool_field') },
      operator: 'Equal',
      right: { type: 'Literal', value: { type: 'Bool', value: true } },
    });
  });

  test('comparison to false', () => {
    const selection = parseSelection('bool_field <> false');
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('bool_field') },
      operator: 'NotEqual',
      right: { type: 'Literal', value: { type: 'Bool', value: false } },
    });
  });

  test('comparison with left operand boolean', () => {
    const selection = parseSelection('false <> bool_field');
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Literal', value: { type: 'Bool', value: false } },
      operator: 'NotEqual',
      right: { type: 'Path', value: PathExpr.simple('bool_field') },
    });
  });

  describe('placeholders', () => {
    test('single literal placeholder in comparison', () => {
      expect(parseSelection('user_id = ?').predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('user_id') },
        operator: 'Equal',
        right: { type: 'Placeholder' },
      });
    });

    test('multiple literal placeholders in AND expression', () => {
      expect(parseSelection('user_id = ? AND status = ?').predicate).toEqual({
        type: 'And',
        left: {
          type: 'Comparison',
          left: { type: 'Path', value: PathExpr.simple('user_id') },
          operator: 'Equal',
          right: { type: 'Placeholder' },
        },
        right: {
          type: 'Comparison',
          left: { type: 'Path', value: PathExpr.simple('status') },
          operator: 'Equal',
          right: { type: 'Placeholder' },
        },
      });
    });

    test('literal placeholders in IN clause', () => {
      expect(parseSelection('status IN (?, ?, ?)').predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('status') },
        operator: 'In',
        right: {
          type: 'ExprList',
          values: [
            { type: 'Placeholder' },
            { type: 'Placeholder' },
            { type: 'Placeholder' },
          ],
        },
      });
    });

    test('predicate placeholders connected by AND', () => {
      expect(parseSelection('? AND ?').predicate).toEqual({
        type: 'And',
        left: { type: 'Placeholder' },
        right: { type: 'Placeholder' },
      });
    });

    test('predicate placeholders connected by OR', () => {
      expect(parseSelection('? OR ?').predicate).toEqual({
        type: 'Or',
        left: { type: 'Placeholder' },
        right: { type: 'Placeholder' },
      });
    });

    test('single predicate placeholder', () => {
      expect(parseSelection('?').predicate).toEqual({ type: 'Placeholder' });
    });

    test('mix of predicate and literal placeholders', () => {
      expect(parseSelection('? AND foo = ?').predicate).toEqual({
        type: 'And',
        left: { type: 'Placeholder' },
        right: {
          type: 'Comparison',
          left: { type: 'Path', value: PathExpr.simple('foo') },
          operator: 'Equal',
          right: { type: 'Placeholder' },
        },
      });
    });
  });

  describe('ORDER BY', () => {
    test('basic ORDER BY', () => {
      const selection = parseSelection("status = 'active' ORDER BY name");
      expect(selection.predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('status') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'String', value: 'active' } },
      });
      expect(selection.orderBy).toEqual([
        { path: PathExpr.simple('name'), direction: 'Asc' },
      ]);
      expect(selection.limit).toBeNull();
    });

    test('ORDER BY with direction', () => {
      const selection = parseSelection('true ORDER BY created_at DESC');
      expect(selection.predicate).toEqual({ type: 'True' });
      expect(selection.orderBy).toEqual([
        { path: PathExpr.simple('created_at'), direction: 'Desc' },
      ]);
    });

    test('ORDER BY dotted identifier not supported', () => {
      expect(() => parseSelection('true ORDER BY user.name ASC')).toThrow();
    });

    test('ORDER BY only', () => {
      const selection = parseSelection('true ORDER BY score');
      expect(selection.predicate).toEqual({ type: 'True' });
      expect(selection.orderBy).toEqual([
        { path: PathExpr.simple('score'), direction: 'Asc' },
      ]);
      expect(selection.limit).toBeNull();
    });

    test('ORDER BY multiple items', () => {
      const selection = parseSelection('true ORDER BY name ASC, created_at DESC, id');
      expect(selection.predicate).toEqual({ type: 'True' });
      expect(selection.orderBy).toEqual([
        { path: PathExpr.simple('name'), direction: 'Asc' },
        { path: PathExpr.simple('created_at'), direction: 'Desc' },
        { path: PathExpr.simple('id'), direction: 'Asc' },
      ]);
      expect(selection.limit).toBeNull();
    });
  });

  describe('LIMIT', () => {
    test('basic LIMIT', () => {
      const selection = parseSelection("status = 'active' LIMIT 10");
      expect(selection.predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('status') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'String', value: 'active' } },
      });
      expect(selection.orderBy).toBeNull();
      expect(selection.limit).toBe(10);
    });

    test('LIMIT only', () => {
      const selection = parseSelection('true LIMIT 100');
      expect(selection.predicate).toEqual({ type: 'True' });
      expect(selection.orderBy).toBeNull();
      expect(selection.limit).toBe(100);
    });
  });

  describe('ORDER BY and LIMIT combined', () => {
    test('both ORDER BY and LIMIT', () => {
      const selection = parseSelection('user_id > 100 ORDER BY created_at DESC LIMIT 5');
      expect(selection.predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('user_id') },
        operator: 'GreaterThan',
        right: { type: 'Literal', value: { type: 'I32', value: 100 } },
      });
      expect(selection.orderBy).toEqual([
        { path: PathExpr.simple('created_at'), direction: 'Desc' },
      ]);
      expect(selection.limit).toBe(5);
    });
  });

  describe('pathological keyword cases', () => {
    test('limit as column name', () => {
      const selection = parseSelection('limit = 1');
      expect(selection.predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('limit') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'I32', value: 1 } },
      });
    });

    test('order as column name with ORDER BY', () => {
      const selection = parseSelection('order = 2 ORDER BY name');
      expect(selection.predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: PathExpr.simple('order') },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'I32', value: 2 } },
      });
      expect(selection.orderBy).toEqual([
        { path: PathExpr.simple('name'), direction: 'Asc' },
      ]);
    });
  });

  describe('boolean literals', () => {
    test('true parses as Predicate.True', () => {
      expect(parseSelection('true').predicate).toEqual({ type: 'True' });
    });

    test('false parses as Predicate.False', () => {
      expect(parseSelection('false').predicate).toEqual({ type: 'False' });
    });
  });

  describe('path expressions', () => {
    test('dotted path in comparison', () => {
      const selection = parseSelection("person.name = 'Alice'");
      expect(selection.predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: new PathExpr(['person', 'name']) },
        operator: 'Equal',
        right: { type: 'Literal', value: { type: 'String', value: 'Alice' } },
      });
    });

    test('dotted paths on both sides', () => {
      const selection = parseSelection('a.foo = b.foo');
      expect(selection.predicate).toEqual({
        type: 'Comparison',
        left: { type: 'Path', value: new PathExpr(['a', 'foo']) },
        operator: 'Equal',
        right: { type: 'Path', value: new PathExpr(['b', 'foo']) },
      });
    });
  });

  describe('case insensitivity', () => {
    test('AND/and/And all work', () => {
      expect(parseSelection("a = 1 AND b = 2").predicate.type).toBe('And');
      expect(parseSelection("a = 1 and b = 2").predicate.type).toBe('And');
      expect(parseSelection("a = 1 And b = 2").predicate.type).toBe('And');
    });

    test('OR/or/Or all work', () => {
      expect(parseSelection("a = 1 OR b = 2").predicate.type).toBe('Or');
      expect(parseSelection("a = 1 or b = 2").predicate.type).toBe('Or');
    });

    test('IS NULL/is null', () => {
      expect(parseSelection("a IS NULL").predicate.type).toBe('IsNull');
      expect(parseSelection("a is null").predicate.type).toBe('IsNull');
    });

    test('TRUE/true/True', () => {
      expect(parseSelection("TRUE").predicate.type).toBe('True');
      expect(parseSelection("true").predicate.type).toBe('True');
    });

    test('IN/in', () => {
      const p1 = parseSelection("x IN (1, 2)").predicate;
      const p2 = parseSelection("x in (1, 2)").predicate;
      expect(p1.type).toBe('Comparison');
      expect(p2.type).toBe('Comparison');
      if (p1.type === 'Comparison' && p2.type === 'Comparison') {
        expect(p1.operator).toBe('In');
        expect(p2.operator).toBe('In');
      }
    });
  });

  test('!= as NotEqual', () => {
    const selection = parseSelection('a != 1');
    expect(selection.predicate).toEqual({
      type: 'Comparison',
      left: { type: 'Path', value: PathExpr.simple('a') },
      operator: 'NotEqual',
      right: { type: 'Literal', value: { type: 'I32', value: 1 } },
    });
  });
});
