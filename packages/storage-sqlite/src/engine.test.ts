// MIRRORS: ankurah/storage/sqlite/src/engine.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { Database } from 'bun:sqlite';
import { Selection, Predicate, parseSelection } from '@ankurah/ankql';
import { CollectionId } from '@ankurah/proto';
import { SqliteStorageEngine, SqlBuilder } from './index.ts';
import type { SqliteDriver } from './index.ts';

// ── bun:sqlite driver adapter ─────────────────────────────────────────

function bunSqliteDriver(path: string = ':memory:'): SqliteDriver {
  const db = new Database(path);

  // Performance optimizations matching Rust connection.rs
  db.exec(
    `PRAGMA journal_mode=WAL;
     PRAGMA synchronous=NORMAL;
     PRAGMA foreign_keys=ON;
     PRAGMA cache_size=-64000;
     PRAGMA mmap_size=268435456;
     PRAGMA temp_store=MEMORY;`,
  );

  return {
    execute(sql: string, params: unknown[] = []): number {
      const stmt = db.prepare(sql);
      const result = stmt.run(...params as any[]);
      return result.changes;
    },
    query<T = Record<string, unknown>>(sql: string, params: unknown[] = []): T[] {
      const stmt = db.prepare(sql);
      return stmt.all(...params as any[]) as T[];
    },
    queryOne<T = Record<string, unknown>>(sql: string, params: unknown[] = []): T | null {
      const stmt = db.prepare(sql);
      const row = stmt.get(...params as any[]);
      return (row as T) ?? null;
    },
    close(): void {
      db.close();
    },
  };
}

describe('SqliteStorageEngine', () => {
  test('test_open_in_memory', async () => {
    const driver = bunSqliteDriver();
    const engine = new SqliteStorageEngine(driver);
    const collection = await engine.collection(CollectionId.from('test_collection'));
    const all = new Selection(Predicate.True(), null, null);
    const results = await collection.fetchStates(all);
    expect(results).toEqual([]);
    driver.close();
  });

  test('test_sane_name', () => {
    expect(SqliteStorageEngine.saneName('test_collection')).toBe(true);
    expect(SqliteStorageEngine.saneName('test.collection')).toBe(true);
    expect(SqliteStorageEngine.saneName('test:collection')).toBe(true);
    expect(SqliteStorageEngine.saneName('test;collection')).toBe(false);
    expect(SqliteStorageEngine.saneName("test'collection")).toBe(false);
  });

  test('test_jsonb_function_availability', () => {
    const driver = bunSqliteDriver();

    // Test 1: Verify jsonb() function exists and works
    const result = driver.queryOne<{ value: Uint8Array }>(
      "SELECT jsonb('{\"key\": \"value\"}') as value",
    );
    expect(result).not.toBeNull();
    // jsonb() returns JSONB BLOB format - verify it's not empty
    expect(result!.value).toBeTruthy();

    // Test 2: Verify json_extract works for path traversal
    const result2 = driver.queryOne<{ value: string }>(
      `SELECT json_extract(jsonb('{"territory": "US", "count": 10}'), '$.territory') as value`,
    );
    expect(result2!.value).toBe('US');

    // Test 3: Verify numeric comparison is numeric (not lexicographic)
    const result3 = driver.queryOne<{ value: number }>(
      `SELECT json_extract(jsonb('{"count": 9}'), '$.count') > json_extract(jsonb('{"count": 10}'), '$.count') as value`,
    );
    expect(result3!.value).toBe(0); // false

    driver.close();
  });

  test('test_json_path_query', () => {
    // Test that the SQL builder generates correct JSONB syntax
    const selection = parseSelection("data.status = 'active'");
    const builder = SqlBuilder.withFields(['id', 'state_buffer']);
    builder.setTableName('test_table');
    builder.selection(selection);

    const [sql] = builder.build();

    // Verify the SQL uses json_extract() for reliable JSON path comparisons
    expect(sql).toContain('json_extract');
    expect(sql).toContain(`json_extract("data", '$.status')`);
  });

  test('test_jsonb_storage_and_parameterized_query', () => {
    const driver = bunSqliteDriver();

    // Create table with BLOB column for JSONB
    driver.execute('CREATE TABLE test_jsonb (id TEXT PRIMARY KEY, data BLOB)');

    // Insert using jsonb(?) - this is what the real code does
    const jsonText = '{"territory": "US", "count": 10}';
    driver.execute(
      'INSERT INTO test_jsonb (id, data) VALUES (?, jsonb(?))',
      ['1', jsonText],
    );

    // Verify data is stored
    const count = driver.queryOne<{ count: number }>('SELECT COUNT(*) as count FROM test_jsonb');
    expect(count!.count).toBe(1);

    // Check what json_extract returns
    const extracted = driver.queryOne<{ value: string }>(
      `SELECT json_extract(data, '$.territory') as value FROM test_jsonb WHERE id = '1'`,
    );
    expect(extracted!.value).toBe('US');

    // Now try the parameterized query - THIS IS WHAT THE REAL CODE DOES
    const queryParam = 'US';
    const result = driver.queryOne<{ id: string }>(
      `SELECT id FROM test_jsonb WHERE json_extract(data, '$.territory') = ?`,
      [queryParam],
    );
    expect(result).not.toBeNull();
    expect(result!.id).toBe('1');

    driver.close();
  });
});
