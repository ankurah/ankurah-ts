// MIRRORS: ankurah/ankql/src/selection/sql.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { generateSelectionSql } from './sql';
import { Result } from '@ankurah/base';
import { ComparisonOperator, Expr, Literal, PathExpr, Predicate } from '../ast';
import { parseSelection } from '../parser';

describe('sql unit tests', () => {
  test('test_simple_equality', () => {
    (() => {
      const selection = parseSelection('name = \'Alice\'').unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, null);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"name" = \'Alice\'');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_and_condition', () => {
    (() => {
      const selection = parseSelection('name = \'Alice\' AND age = \'30\'').unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, null);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"name" = \'Alice\' AND "age" = \'30\'');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_complex_condition', () => {
    (() => {
      const selection = parseSelection('(name = \'Alice\' OR name = \'Charlie\') AND age >= \'30\' AND age <= \'40\'').unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, null);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('("name" = \'Alice\' OR "name" = \'Charlie\') AND "age" >= \'30\' AND "age" <= \'40\'');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_including_collection_identifier', () => {
    (() => {
      const selection = parseSelection('person.name = \'Alice\'').unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, null);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"person"."name" = \'Alice\'');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_in_operator', () => {
    (() => {
      const selection = parseSelection('name IN (\'Alice\', \'Bob\', \'Charlie\')').unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, null);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"name" IN (\'Alice\', \'Bob\', \'Charlie\')');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_placeholder_with_none_count', () => {
    (() => {
      const query = 'user_id = ?';
      const selection = parseSelection(query).unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, null);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"user_id" = ?');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_placeholder_with_exact_count', () => {
    (() => {
      const query = 'user_id = ? AND status = ?';
      const selection = parseSelection(query).unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, 2);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"user_id" = ? AND "status" = ?');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_placeholder_count_mismatch_too_few', () => {
    (() => {
      const _r0 = parseSelection('user_id = ? AND status = ?');
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const selection = _r0.unwrap();
      try {
        const _v = generateSelectionSql(selection.predicate, 1);
        if (_v.isOk()) {
          const _v2 = _v.unwrap();
          throw new Error('Expected PlaceholderCountMismatch error')
        } else {
          const _v1 = _v.unwrapErr();
          (() => {
            expect(expected).toEqual(1);
            expect(found).toEqual(2);
          })()
        }
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_placeholder_count_mismatch_too_many', () => {
    (() => {
      const _r0 = parseSelection('user_id = ?');
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const selection = _r0.unwrap();
      try {
        const _v = generateSelectionSql(selection.predicate, 2);
        if (_v.isOk()) {
          const _v2 = _v.unwrap();
          throw new Error('Expected PlaceholderCountMismatch error')
        } else {
          const _v1 = _v.unwrapErr();
          (() => {
            expect(expected).toEqual(2);
            expect(found).toEqual(1);
          })()
        }
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_placeholder_in_lists', () => {
    (() => {
      const query = 'status IN (?, ?, ?)';
      const selection = parseSelection(query).unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, 3);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"status" IN (?, ?, ?)');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_placeholder_with_zero_count', () => {
    (() => {
      const query = 'user_id = 123';
      const selection = parseSelection(query).unwrap();
      try {
        const _r0 = generateSelectionSql(selection.predicate, 0);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"user_id" = 123');
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

  test('test_string_escaping', () => {
    (() => {
      const predicate = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('name') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'O\'Brien' }) }) });
      try {
        const _r0 = generateSelectionSql(predicate, null);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"name" = \'O\'\'Brien\'');
        return Result.Ok([]);
      } finally {
        predicate.drop();
      }
    })().unwrap();
  });

  test('test_null_byte_handling', () => {
    (() => {
      const predicate = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('data') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'test\u{0}data' }) }) });
      try {
        const _r0 = generateSelectionSql(predicate, null);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const sql = _r0.unwrap();
        expect(sql).toEqual('"data" = \'testdata\'');
        return Result.Ok([]);
      } finally {
        predicate.drop();
      }
    })().unwrap();
  });

  test('test_placeholder_with_zero_count_but_has_placeholder', () => {
    (() => {
      const _r0 = parseSelection('user_id = ?');
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const selection = _r0.unwrap();
      try {
        const _v = generateSelectionSql(selection.predicate, 0);
        if (_v.isOk()) {
          const _v2 = _v.unwrap();
          throw new Error('Expected PlaceholderCountMismatch error')
        } else {
          const _v1 = _v.unwrapErr();
          (() => {
            expect(expected).toEqual(0);
            expect(found).toEqual(1);
          })()
        }
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    })().unwrap();
  });

});
