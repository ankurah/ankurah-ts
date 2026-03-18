// MIRRORS: ankurah/storage/indexeddb-wasm/tests/edge_cases.rs

import { describe, test, expect } from 'bun:test';
import {
  createIndexedDBNode, createAlbums, createBooks, Album, Book,
  names, sortNames, years, matchArgs, IndexedDBStorageEngine,
} from './common.ts';

describe('edge_cases', () => {
  test('test_edge_cases', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    // Create test data with edge cases
    await createAlbums(ctx, [
      ['', '2000'],                                           // Empty name
      ['Album with spaces', '2001'],                          // Spaces
      ['Album-with-dashes', '2002'],                          // Dashes
      ['Album_with_underscores', '2003'],                     // Underscores
      ['UPPERCASE', '2004'],                                  // Case variations
      ['lowercase', '2005'],
      ['MixedCase', '2006'],
      ['Special!@#$%', '2007'],                               // Special characters
      ['Unicode: 你好', '2008'],                               // Unicode
      ['Very Long Album Name That Goes On And On And On', '2009'], // Long name
    ]);

    // Test empty string handling
    expect(names(await ctx.fetch(Album, matchArgs("name = ''")))).toEqual(['']);

    // Test special characters in queries
    expect(names(await ctx.fetch(Album, matchArgs("name = 'Special!@#$%'")))).toEqual(['Special!@#$%']);

    // Test Unicode support
    expect(names(await ctx.fetch(Album, matchArgs("name = 'Unicode: 你好'")))).toEqual(['Unicode: 你好']);

    // Test case sensitivity
    expect(names(await ctx.fetch(Album, matchArgs("name = 'UPPERCASE'")))).toEqual(['UPPERCASE']);
    expect(names(await ctx.fetch(Album, matchArgs("name = 'lowercase'")))).toEqual(['lowercase']);

    // Test complex AND/OR combinations
    expect(
      sortNames(await ctx.fetch(Album, matchArgs("(name = 'UPPERCASE' OR name = 'lowercase') AND year >= '2004'"))),
    ).toEqual(['UPPERCASE', 'lowercase']);

    // Test range queries with string comparison edge cases
    expect(years(await ctx.fetch(Album, matchArgs("year > '2005' AND year < '2008'")))).toEqual(['2006', '2007']);

    // Test impossible range (conflicting inequalities) - should return empty results, not crash
    expect(names(await ctx.fetch(Album, matchArgs("year > '2010' AND year < '2005'")))).toEqual([]);

    // Test ordering with special characters and case
    expect(
      names(await ctx.fetch(Album, matchArgs("year >= '2001' ORDER BY name LIMIT 5"))),
    ).toEqual(['Album with spaces', 'Album-with-dashes', 'Album_with_underscores', 'MixedCase', 'Special!@#$%']);

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_prefix_guard_collection_boundary', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    // Insert albums and books with overlapping names to ensure sorted adjacency
    await createAlbums(ctx, [
      ['Album1', '1965'], ['Album2', '1966'], ['Album3', '1967'],
      ['Album4', '1968'], ['Album5', '1969'], ['Album6', '1970'],
    ]);
    await createBooks(ctx, [['Book1', '2001'], ['Book2', '2002']]);

    // ORDER-FIRST plan over (__collection, name), now with bounded __collection range
    // LIMIT 5 should only include album records, never book
    expect(
      names(await ctx.fetch(Album, matchArgs("year >= '1900' ORDER BY name LIMIT 5"))),
    ).toEqual(['Album1', 'Album2', 'Album3', 'Album4', 'Album5']);

    // Larger limit should still exclude books due to bounded range
    expect(
      names(await ctx.fetch(Album, matchArgs("year >= '1900' ORDER BY name LIMIT 100"))),
    ).toEqual(['Album1', 'Album2', 'Album3', 'Album4', 'Album5', 'Album6']);

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });
});
