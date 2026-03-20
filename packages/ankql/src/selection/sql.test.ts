// MIRRORS: ankurah/ankql/src/selection/sql.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { generateSelectionSql } from './sql.ts';
import { parseSelection } from '../parser.ts';
import { PathExpr, Predicate, Expr, Literal, ComparisonOperator } from '../ast.ts';
import { SqlGenerationError } from '../error.ts';

describe('sql generation', () => {
  // Rust: fn test_simple_equality
  test('simple equality', () => {
    using selection = parseSelection("name = 'Alice'");
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"name" = \'Alice\'');
  });

  // Rust: fn test_and_condition
  test('AND condition', () => {
    using selection = parseSelection("name = 'Alice' AND age = '30'");
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"name" = \'Alice\' AND "age" = \'30\'');
  });

  // Rust: fn test_complex_condition
  test('complex condition', () => {
    using selection = parseSelection(
      "(name = 'Alice' OR name = 'Charlie') AND age >= '30' AND age <= '40'",
    );
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe(
      '("name" = \'Alice\' OR "name" = \'Charlie\') AND "age" >= \'30\' AND "age" <= \'40\'',
    );
  });

  // Rust: fn test_including_collection_identifier
  test('including collection identifier (dotted path)', () => {
    using selection = parseSelection("person.name = 'Alice'");
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"person"."name" = \'Alice\'');
  });

  // Rust: fn test_in_operator
  test('IN operator', () => {
    using selection = parseSelection("name IN ('Alice', 'Bob', 'Charlie')");
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"name" IN (\'Alice\', \'Bob\', \'Charlie\')');
  });

  // Rust: fn test_placeholder_with_none_count
  test('placeholder with None count', () => {
    using selection = parseSelection('user_id = ?');
    const sql = generateSelectionSql(selection.predicate);
    expect(sql).toBe('"user_id" = ?');
  });

  // Rust: fn test_placeholder_with_exact_count
  test('placeholder with exact count', () => {
    using selection = parseSelection('user_id = ? AND status = ?');
    const sql = generateSelectionSql(selection.predicate, 2);
    expect(sql).toBe('"user_id" = ? AND "status" = ?');
  });

  // Rust: fn test_placeholder_count_mismatch_too_few
  test('placeholder count mismatch: too few expected', () => {
    using selection = parseSelection('user_id = ? AND status = ?');
    try {
      generateSelectionSql(selection.predicate, 1);
      expect(true).toBe(false); // should not reach here
    } catch (e) {
      expect(e).toBeInstanceOf(SqlGenerationError);
      if (e instanceof SqlGenerationError) {
        expect(e.type).toBe('PlaceholderCountMismatch');
        const v = e.value as { expected: number; found: number };
        expect(v.expected).toBe(1);
        expect(v.found).toBe(2);
      }
    }
  });

  // Rust: fn test_placeholder_count_mismatch_too_many
  test('placeholder count mismatch: too many expected', () => {
    using selection = parseSelection('user_id = ?');
    try {
      generateSelectionSql(selection.predicate, 2);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(SqlGenerationError);
      if (e instanceof SqlGenerationError) {
        expect(e.type).toBe('PlaceholderCountMismatch');
        const v = e.value as { expected: number; found: number };
        expect(v.expected).toBe(2);
        expect(v.found).toBe(1);
      }
    }
  });

  // Rust: fn test_placeholder_in_lists
  test('placeholder in lists', () => {
    using selection = parseSelection('status IN (?, ?, ?)');
    const sql = generateSelectionSql(selection.predicate, 3);
    expect(sql).toBe('"status" IN (?, ?, ?)');
  });

  // Rust: fn test_placeholder_with_zero_count
  test('placeholder with zero count (no placeholders)', () => {
    using selection = parseSelection('user_id = 123');
    const sql = generateSelectionSql(selection.predicate, 0);
    expect(sql).toBe('"user_id" = 123');
  });

  // Rust: fn test_string_escaping
  test('string escaping: single quotes', () => {
    using predicate = Predicate.Comparison(
      Expr.Path(PathExpr.simple('name')),
      ComparisonOperator.Equal(),
      Expr.Literal(Literal.String("O'Brien")),
    );
    const sql = generateSelectionSql(predicate);
    expect(sql).toBe('"name" = \'O\'\'Brien\'');
  });

  // Rust: fn test_null_byte_handling
  test('null byte handling', () => {
    using predicate = Predicate.Comparison(
      Expr.Path(PathExpr.simple('data')),
      ComparisonOperator.Equal(),
      Expr.Literal(Literal.String('test\0data')),
    );
    const sql = generateSelectionSql(predicate);
    expect(sql).toBe('"data" = \'testdata\'');
  });

  // Rust: fn test_placeholder_with_zero_count_but_has_placeholder
  test('placeholder with zero count but has placeholder', () => {
    using selection = parseSelection('user_id = ?');
    try {
      generateSelectionSql(selection.predicate, 0);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(SqlGenerationError);
      if (e instanceof SqlGenerationError) {
        expect(e.type).toBe('PlaceholderCountMismatch');
        const v = e.value as { expected: number; found: number };
        expect(v.expected).toBe(0);
        expect(v.found).toBe(1);
      }
    }
  });
});
