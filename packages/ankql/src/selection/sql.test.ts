// MIRRORS: ankurah/ankql/src/selection/sql.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { generateSelectionSql } from './sql.ts';
import { parseSelection } from '../parser.ts';
import { PathExpr, Predicate, Expr, Literal, ComparisonOperator } from '../ast.ts';
import { PlaceholderCountMismatchError } from '../error.ts';

describe('sql generation', () => {
  test('simple equality', () => {
    const selection = parseSelection("name = 'Alice'");
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"name" = \'Alice\'');
  });

  test('AND condition', () => {
    const selection = parseSelection("name = 'Alice' AND age = '30'");
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"name" = \'Alice\' AND "age" = \'30\'');
  });

  test('complex condition', () => {
    const selection = parseSelection(
      "(name = 'Alice' OR name = 'Charlie') AND age >= '30' AND age <= '40'",
    );
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe(
      '("name" = \'Alice\' OR "name" = \'Charlie\') AND "age" >= \'30\' AND "age" <= \'40\'',
    );
  });

  test('including collection identifier (dotted path)', () => {
    const selection = parseSelection("person.name = 'Alice'");
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"person"."name" = \'Alice\'');
  });

  test('IN operator', () => {
    const selection = parseSelection("name IN ('Alice', 'Bob', 'Charlie')");
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"name" IN (\'Alice\', \'Bob\', \'Charlie\')');
  });

  test('placeholder with None count', () => {
    const selection = parseSelection('user_id = ?');
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"user_id" = ?');
  });

  test('placeholder with exact count', () => {
    const selection = parseSelection('user_id = ? AND status = ?');
    const sql = generateSelectionSql(selection.predicate, 2);
    expect(sql).toBe('"user_id" = ? AND "status" = ?');
  });

  test('placeholder count mismatch: too few expected', () => {
    const selection = parseSelection('user_id = ? AND status = ?');
    try {
      generateSelectionSql(selection.predicate, 1);
      expect(true).toBe(false); // should not reach here
    } catch (e) {
      expect(e).toBeInstanceOf(PlaceholderCountMismatchError);
      if (e instanceof PlaceholderCountMismatchError) {
        expect(e.expected).toBe(1);
        expect(e.found).toBe(2);
      }
    }
  });

  test('placeholder count mismatch: too many expected', () => {
    const selection = parseSelection('user_id = ?');
    try {
      generateSelectionSql(selection.predicate, 2);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(PlaceholderCountMismatchError);
      if (e instanceof PlaceholderCountMismatchError) {
        expect(e.expected).toBe(2);
        expect(e.found).toBe(1);
      }
    }
  });

  test('placeholder in lists', () => {
    const selection = parseSelection('status IN (?, ?, ?)');
    const sql = generateSelectionSql(selection.predicate, 3);
    expect(sql).toBe('"status" IN (?, ?, ?)');
  });

  test('placeholder with zero count (no placeholders)', () => {
    const selection = parseSelection('user_id = 123');
    const sql = generateSelectionSql(selection.predicate, 0);
    expect(sql).toBe('"user_id" = 123');
  });

  test('string escaping: single quotes', () => {
    const predicate = Predicate.Comparison(
      Expr.Path(PathExpr.simple('name')),
      ComparisonOperator.Equal(),
      Expr.Literal(Literal.String("O'Brien")),
    );
    const sql = generateSelectionSql(predicate);
    expect(sql).toBe('"name" = \'O\'\'Brien\'');
  });

  test('null byte handling', () => {
    const predicate = Predicate.Comparison(
      Expr.Path(PathExpr.simple('data')),
      ComparisonOperator.Equal(),
      Expr.Literal(Literal.String('test\0data')),
    );
    const sql = generateSelectionSql(predicate);
    expect(sql).toBe('"data" = \'testdata\'');
  });

  test('placeholder with zero count but has placeholder', () => {
    const selection = parseSelection('user_id = ?');
    try {
      generateSelectionSql(selection.predicate, 0);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(PlaceholderCountMismatchError);
      if (e instanceof PlaceholderCountMismatchError) {
        expect(e.expected).toBe(0);
        expect(e.found).toBe(1);
      }
    }
  });
});
