// MIRRORS: ankurah/storage/sqlite/src/engine.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { SqliteStorageEngine } from './engine';
import { Result, dropOwned } from '@ankurah/base';
import { SqliteError } from './error';
import { SqlBuilder } from './sql_builder';
import { Predicate, Selection, parseSelection } from '@ankurah/ankql';

describe('engine unit tests', () => {
  test('test_open_in_memory', async () => {
    const engine = (await SqliteStorageEngine.openInMemory()).unwrap();
    try {
      const collection = (await engine.collection('test_collection')).unwrap();
      try {
        const all = new Selection(new Predicate('True', {}), null, null);
        try {
          const _t0 = (await collection.value.fetchStates(all)).unwrap();
          try {
            if (!(_t0.length === 0)) throw new Error('assertion failed');
          } finally {
            dropOwned(_t0);
          }
        } finally {
          all.drop();
        }
      } finally {
        collection.drop();
      }
    } finally {
      engine.drop();
    }
  });

  test('test_sane_name', async () => {
    if (!(SqliteStorageEngine.saneName('test_collection'))) throw new Error('assertion failed');
    if (!(SqliteStorageEngine.saneName('test.collection'))) throw new Error('assertion failed');
    if (!(SqliteStorageEngine.saneName('test:collection'))) throw new Error('assertion failed');
    if (!(!SqliteStorageEngine.saneName('test;collection'))) throw new Error('assertion failed');
    if (!(!SqliteStorageEngine.saneName('test\'collection'))) throw new Error('assertion failed');
  });

  test('test_jsonb_function_availability', async () => {
    (await (async () => {
      const _r0 = (await SqliteStorageEngine.openInMemory()).mapErr((e) => new SqliteError('DDL', { _0: e.toString() }));
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const engine = _r0.unwrap();
      try {
        const _r1 = (await engine.pool.get()).mapErr((e) => new SqliteError('Pool', { _0: e.toString() }));
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        const conn = _r1.unwrap();
        try {
          const _r3 = await conn.withConnection((c) => {
            const _r2 = c.queryRow('SELECT jsonb(\'{"key": "value"}\')', [], (row) => row.get(0));
            if (_r2.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r2.unwrapErr()));
            const value = _r2.unwrap();
            return Result.Ok(value);
          });
          if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
          const result = _r3.unwrap();
          if (!(!result.isEmpty())) throw new Error('jsonb() function should return a non-empty BLOB');
          const _r5 = await conn.withConnection((c) => {
            const _r4 = c.queryRow('SELECT json_extract(jsonb(\'{"territory": "US", "count": 10}\'), \'$.territory\')', [], (row) => row.get(0));
            if (_r4.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r4.unwrapErr()));
            const value = _r4.unwrap();
            return Result.Ok(value);
          });
          if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
          const result_1 = _r5.unwrap();
          expect(result_1).toEqual('US');
          const _r7 = await conn.withConnection((c) => {
            const _r6 = c.queryRow('SELECT json_extract(jsonb(\'{"count": 9}\'), \'$.count\') > json_extract(jsonb(\'{"count": 10}\'), \'$.count\')', [], (row) => row.get(0));
            if (_r6.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r6.unwrapErr()));
            const value = _r6.unwrap();
            return Result.Ok(value);
          });
          if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
          const result_2 = _r7.unwrap();
          if (!(!result_2)) throw new Error('Numeric comparison: 9 > 10 should be false');
          return Result.Ok([]);
        } finally {
          dropOwned(conn);
        }
      } finally {
        engine.drop();
      }
    })()).unwrap();
  });

  test('test_json_path_query', async () => {
    (await (async () => {
      const selection = parseSelection('data.status = \'active\'');
      let _moved0 = false;
      let builder = SqlBuilder.withFields(['id', 'state_buffer']);
      try {
        builder.tableName('test_table');
        const _r1 = builder.selection(selection).mapErr((e) => new SqliteError('SqlGeneration', { _0: e.toString() }));
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        _r1.drop();
        _moved0 = true;
        const _r2 = builder.build().mapErr((e) => new SqliteError('SqlGeneration', { _0: e.toString() }));
        if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
        const [sql, _params] = _r2.unwrap();
        if (!(sql.includes('json_extract'))) throw new Error(`SQL should use json_extract() for JSON path: ${sql}`);
        if (!(sql.includes('json_extract("data", \'$.status\')'))) throw new Error(`SQL should extract from data column with $.status path: ${sql}`);
        return Result.Ok([]);
      } finally {
        if (!_moved0) builder.drop();
      }
    })()).unwrap();
  });

  test('test_jsonb_storage_and_parameterized_query', async () => {
    (await (async () => {
      const _r0 = (await SqliteStorageEngine.openInMemory()).mapErr((e) => new SqliteError('DDL', { _0: e.toString() }));
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const engine = _r0.unwrap();
      try {
        const _r1 = (await engine.pool.get()).mapErr((e) => new SqliteError('Pool', { _0: e.toString() }));
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        const conn = _r1.unwrap();
        try {
          return await conn.withConnection((c) => {
            const _r2 = c.execute('CREATE TABLE test_jsonb (id TEXT PRIMARY KEY, data BLOB)', []);
            if (_r2.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r2.unwrapErr()));
            _r2.drop();
            const jsonText = '{"territory": "US", "count": 10}';
            const _r3 = c.execute('INSERT INTO test_jsonb (id, data) VALUES (?, jsonb(?))', ['1', jsonText]);
            if (_r3.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r3.unwrapErr()));
            _r3.drop();
            const _r4 = c.queryRow('SELECT COUNT(*) FROM test_jsonb', [], (row) => row.get(0));
            if (_r4.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r4.unwrapErr()));
            const count = _r4.unwrap();
            expect(count).toEqual(1);
            const _r5 = c.queryRow('SELECT typeof(data) FROM test_jsonb WHERE id = \'1\'', [], (row) => row.get(0));
            if (_r5.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r5.unwrapErr()));
            const dataType = _r5.unwrap();
            console.log(`Data column type: ${dataType}`);
            const _r6 = c.queryRow('SELECT json_extract(data, \'$.territory\') FROM test_jsonb WHERE id = \'1\'', [], (row) => row.get(0));
            if (_r6.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r6.unwrapErr()));
            const extracted = _r6.unwrap();
            console.log(`Extracted territory: '${extracted}'`);
            const queryParam = 'US';
            const result = c.queryRow('SELECT id FROM test_jsonb WHERE json_extract(data, \'$.territory\') = ?', [queryParam], (row) => row.get(0));
            try {
              console.log(`Query result: ${result}`);
              if (result.isOk()) {
                const id = result.unwrap();
                expect(id).toEqual('1')
              } else {
                const e = result.unwrapErr();
                throw new Error(`Query failed: ${e}`)
              }
              return Result.Ok([]);
            } finally {
              result.drop();
            }
          });
        } finally {
          dropOwned(conn);
        }
      } finally {
        engine.drop();
      }
    })()).unwrap();
  });

});
