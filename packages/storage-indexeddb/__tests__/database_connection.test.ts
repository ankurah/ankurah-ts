// MIRRORS: ankurah/storage/indexeddb-wasm/tests/database_connection.rs

import { describe, test, expect } from 'bun:test';
import 'fake-indexeddb/auto';
import { IndexedDBStorageEngine } from '../src/index.ts';
import { Database } from '../src/database.ts';
import { keySpecNew, indexKeyPartAsc, ValueType } from '@ankurah/core';

describe('database_connection', () => {
  test('test_open_database', async () => {
    const dbName = `test_db_open_${Date.now()}`;
    const engine = await IndexedDBStorageEngine.open(dbName);
    expect(engine.name()).toBe(dbName);

    const engine2 = await IndexedDBStorageEngine.open(dbName);
    expect(engine2.name()).toBe(dbName);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_multi_connection_versionchange_reconnect', async () => {
    const dbName = `test_db_multi_conn_${Date.now()}`;

    // Open two logical connections via engine wrappers
    const engine1 = await IndexedDBStorageEngine.open(dbName);
    const engine2 = await IndexedDBStorageEngine.open(dbName);

    const db1 = await Database.open(dbName);
    const versionBefore = (await db1.getConnection()).version;

    // Trigger an upgrade via assureIndexExists
    const indexSpec = keySpecNew([indexKeyPartAsc('multi_conn_field', ValueType.String)]);
    await db1.assureIndexExists(indexSpec);

    // Other connections should have received versionchange and closed; lazy reconnect should yield newer version
    const db2 = await Database.open(dbName);
    const v1 = (await db1.getConnection()).version;
    const v2 = (await db2.getConnection()).version;
    expect(v1).toBeGreaterThanOrEqual(versionBefore + 1);
    expect(v2).toBeGreaterThanOrEqual(versionBefore + 1);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_duplicate_index_creation_error_handling', async () => {
    const dbName = `test_db_duplicate_index_${Date.now()}`;
    const indexSpec = keySpecNew([indexKeyPartAsc('test_field', ValueType.String)]);

    // First, create the database and establish a baseline version
    const db = await Database.open(dbName);
    const initialVersion = (await db.getConnection()).version;
    db.close();

    // First call - should succeed and create the index
    const db2 = await Database.open(dbName);
    await db2.assureIndexExists(indexSpec);
    db2.close();

    // Second call with a fresh connection - assureIndexExists sees the index already exists
    // and should not error (it's idempotent via the cache/hasIndex check)
    const db3 = await Database.open(dbName);
    await db3.assureIndexExists(indexSpec);
    db3.close();

    await IndexedDBStorageEngine.cleanup(dbName);
  });
});
