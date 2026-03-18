// MIRRORS: ankurah/storage/indexeddb-wasm/tests/index_creation.rs

import { describe, test, expect } from 'bun:test';
import 'fake-indexeddb/auto';
import { IndexedDBStorageEngine } from '../src/index.ts';
import { Database } from '../src/database.ts';
import { keySpecNew, indexKeyPartAsc, keySpecNameWith, ValueType } from '@ankurah/core';

describe('index_creation', () => {
  test('test_index_creation_and_reconnection', async () => {
    const dbName = `test_index_${Date.now()}`;
    const engine = await IndexedDBStorageEngine.open(dbName);

    // Get the Database instance from the storage engine
    const db = engine.db;

    // Test that we can get a connection initially
    const initialVersion = (await db.getConnection()).version;

    // Create an index spec for testing
    const indexSpec = keySpecNew([
      indexKeyPartAsc('__collection', ValueType.String),
      indexKeyPartAsc('name', ValueType.String),
    ]);

    // Test index creation (this should trigger reconnection)
    await db.assureIndexExists(indexSpec);

    // Verify we can still get a connection after index creation
    const postIndexVersion = (await db.getConnection()).version;

    // Version should have been incremented
    expect(postIndexVersion).toBeGreaterThan(initialVersion);

    // Verify we can create a transaction on the new connection and access the index
    const conn = await db.getConnection();
    const transaction = conn.transaction('entities', 'readonly');
    const store = transaction.objectStore('entities');

    // Verify the index exists by trying to access it
    const indexName = keySpecNameWith(indexSpec, '', '__');
    const index = store.index(indexName);
    expect(index).toBeDefined();
    expect(index.name).toBe(indexName);

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });
});
