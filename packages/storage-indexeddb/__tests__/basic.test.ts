// MIRRORS: ankurah/storage/indexeddb-wasm/tests/basic.rs

import { describe, test, expect } from 'bun:test';
import {
  createIndexedDBNode, createAlbums, Album, names, years,
  matchArgs, IndexedDBStorageEngine,
} from './common.ts';

describe('basic', () => {
  test('test_indexeddb_basic_workflow', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    // Create a simple test - just verify IndexedDB storage works
    await createAlbums(ctx, [['Walking on a Dream', '2008']]);

    // Verify we can query the album
    const byName = await ctx.fetch(Album, matchArgs("name = 'Walking on a Dream'"));
    expect(names(byName)).toEqual(['Walking on a Dream']);
    expect(years(byName)).toEqual(['2008']);

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });
});
