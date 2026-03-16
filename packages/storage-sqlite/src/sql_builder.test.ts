// MIRRORS: ankurah/storage/sqlite/src/sql_builder.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { parseSelection } from '@ankurah/ankql';
import { SqlBuilder } from './sql_builder.ts';

describe('SqlBuilder', () => {
  test('test_simple_equality', () => {
    const selection = parseSelection("name = 'Alice'");
    const sql = SqlBuilder.new();
    sql.selection(selection);
    const [sqlString, params] = sql.buildWhereClause();

    expect(sqlString).toBe('"name" = ?');
    expect(params.length).toBe(1);
  });

  test('test_and_condition', () => {
    const selection = parseSelection("name = 'Alice' AND age = 30");
    const sql = SqlBuilder.withFields(['id', 'name', 'age']);
    sql.setTableName('users');
    sql.selection(selection);
    const [sqlString, params] = sql.build();

    expect(sqlString).toBe('SELECT "id", "name", "age" FROM "users" WHERE "name" = ? AND "age" = ?');
    expect(params.length).toBe(2);
  });

  test('test_json_path', () => {
    const selection = parseSelection("data.status = 'active'");
    const sql = SqlBuilder.new();
    sql.selection(selection);
    const [sqlString] = sql.buildWhereClause();

    expect(sqlString).toBe('json_extract("data", \'$.status\') = ?');
  });

  test('test_json_nested_path', () => {
    const selection = parseSelection("data.user.name = 'Alice'");
    const sql = SqlBuilder.new();
    sql.selection(selection);
    const [sqlString] = sql.buildWhereClause();

    expect(sqlString).toBe('json_extract("data", \'$.user.name\') = ?');
  });

  test('test_json_numeric_comparison', () => {
    const selection = parseSelection("data.count > 10");
    const sql = SqlBuilder.new();
    sql.selection(selection);
    const [sqlString] = sql.buildWhereClause();

    expect(sqlString).toBe('json_extract("data", \'$.count\') > ?');
  });

  test('test_in_operator', () => {
    const selection = parseSelection("name IN ('Alice', 'Bob')");
    const sql = SqlBuilder.new();
    sql.selection(selection);
    const [sqlString, params] = sql.buildWhereClause();

    expect(sqlString).toBe('"name" IN (?, ?)');
    expect(params.length).toBe(2);
  });
});
