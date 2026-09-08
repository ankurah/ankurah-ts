// MIRRORS: ankurah/storage/sqlite/src/sql_builder.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { SqlBuilder } from './sql_builder';
import { parseSelection } from '@ankurah/ankql';

describe('sql_builder unit tests', () => {
  test('test_simple_equality', () => {
    const selection = parseSelection('name = \'Alice\'').unwrap();
    try {
      let _moved0 = false;
      let sql = SqlBuilder.new();
      try {
        sql.selection(selection).unwrap();
        _moved0 = true;
        const [sqlString, params] = sql.buildWhereClause();
        expect(sqlString).toEqual('"name" = ?');
        expect(params.length).toEqual(1);
      } finally {
        if (!_moved0) sql.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_and_condition', () => {
    const selection = parseSelection('name = \'Alice\' AND age = 30').unwrap();
    try {
      let _moved0 = false;
      let sql = SqlBuilder.withFields(['id', 'name', 'age']);
      try {
        sql.tableName('users');
        sql.selection(selection).unwrap();
        _moved0 = true;
        const [sqlString, params] = sql.build().unwrap();
        expect(sqlString).toEqual('SELECT "id", "name", "age" FROM "users" WHERE "name" = ? AND "age" = ?');
        expect(params.length).toEqual(2);
      } finally {
        if (!_moved0) sql.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_json_path', () => {
    const selection = parseSelection('data.status = \'active\'').unwrap();
    try {
      let _moved0 = false;
      let sql = SqlBuilder.new();
      try {
        sql.selection(selection).unwrap();
        _moved0 = true;
        const [sqlString, ] = sql.buildWhereClause();
        expect(sqlString).toEqual('json_extract("data", \'$.status\') = ?');
      } finally {
        if (!_moved0) sql.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_json_nested_path', () => {
    const selection = parseSelection('data.user.name = \'Alice\'').unwrap();
    try {
      let _moved0 = false;
      let sql = SqlBuilder.new();
      try {
        sql.selection(selection).unwrap();
        _moved0 = true;
        const [sqlString, ] = sql.buildWhereClause();
        expect(sqlString).toEqual('json_extract("data", \'$.user.name\') = ?');
      } finally {
        if (!_moved0) sql.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_json_numeric_comparison', () => {
    const selection = parseSelection('data.count > 10').unwrap();
    try {
      let _moved0 = false;
      let sql = SqlBuilder.new();
      try {
        sql.selection(selection).unwrap();
        _moved0 = true;
        const [sqlString, ] = sql.buildWhereClause();
        expect(sqlString).toEqual('json_extract("data", \'$.count\') > ?');
      } finally {
        if (!_moved0) sql.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_in_operator', () => {
    const selection = parseSelection('name IN (\'Alice\', \'Bob\')').unwrap();
    try {
      let _moved0 = false;
      let sql = SqlBuilder.new();
      try {
        sql.selection(selection).unwrap();
        _moved0 = true;
        const [sqlString, params] = sql.buildWhereClause();
        expect(sqlString).toEqual('"name" IN (?, ?)');
        expect(params.length).toEqual(2);
      } finally {
        if (!_moved0) sql.drop();
      }
    } finally {
      selection.drop();
    }
  });

});
