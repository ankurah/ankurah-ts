// MIRRORS: ankurah/storage/indexeddb-wasm/tests/json_property.rs

import { describe, test, expect } from 'bun:test';
import {
  createIndexedDBNode, createTracks, Track,
  names, matchArgs, IndexedDBStorageEngine,
} from './common.ts';

describe('json_property', () => {
  test('test_json_property_storage_and_simple_query', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createTracks(ctx, [
      ['Test Track', { territory: 'US', rights: 'exclusive' }],
    ]);

    // Simple query (non-JSON) - should work
    const tracks = await ctx.fetch(Track, matchArgs("name = 'Test Track'"));
    expect(tracks.length).toBe(1);
    expect(tracks[0].name()).toBe('Test Track');

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_json_path_query_string_equality', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createTracks(ctx, [
      ['US Track', { territory: 'US', rights: 'exclusive' }],
      ['UK Track', { territory: 'UK', rights: 'non-exclusive' }],
    ]);

    // Query by JSON path
    const usTracks = await ctx.fetch(Track, matchArgs("licensing.territory = 'US'"));
    expect(usTracks.length).toBe(1);
    expect(usTracks[0].name()).toBe('US Track');

    const ukTracks = await ctx.fetch(Track, matchArgs("licensing.territory = 'UK'"));
    expect(ukTracks.length).toBe(1);
    expect(ukTracks[0].name()).toBe('UK Track');

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_json_path_query_numeric_comparison', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createTracks(ctx, [
      ['Popular Track', { territory: 'US', plays: 1000 }],
      ['New Track', { territory: 'US', plays: 50 }],
    ]);

    // Query with numeric comparison
    const popular = await ctx.fetch(Track, matchArgs('licensing.plays > 500'));
    expect(popular.length).toBe(1);
    expect(popular[0].name()).toBe('Popular Track');

    // Equality
    const exact = await ctx.fetch(Track, matchArgs('licensing.plays = 1000'));
    expect(exact.length).toBe(1);
    expect(exact[0].name()).toBe('Popular Track');

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_json_path_nested_query', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createTracks(ctx, [
      ['Nested Track', { territory: 'US', rights: { holder: 'Label', type: 'exclusive' } }],
    ]);

    // Query nested path
    const labelTracks = await ctx.fetch(Track, matchArgs("licensing.rights.holder = 'Label'"));
    expect(labelTracks.length).toBe(1);
    expect(labelTracks[0].name()).toBe('Nested Track');

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_json_path_combined_with_regular_field', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createTracks(ctx, [
      ['US Track A', { territory: 'US' }],
      ['US Track B', { territory: 'US' }],
      ['UK Track', { territory: 'UK' }],
    ]);

    // Query combining regular field and JSON path
    const results = await ctx.fetch(Track, matchArgs("name = 'US Track A' AND licensing.territory = 'US'"));
    expect(results.length).toBe(1);
    expect(results[0].name()).toBe('US Track A');

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_json_path_missing_field', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createTracks(ctx, [
      ['Has Territory', { territory: 'US' }],
      ['No Territory', { other: 'value' }], // Missing territory field
    ]);

    // Query should only find the track that HAS the territory field
    const results = await ctx.fetch(Track, matchArgs("licensing.territory = 'US'"));
    expect(results.length).toBe(1);
    expect(results[0].name()).toBe('Has Territory');

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_json_path_planner_generates_sub_path', () => {
    // Sync test verifying planner behavior
    const { Planner, plannerConfigIndexeddb, Plan } = require('@ankurah/storage-common');
    const { parseSelection } = require('@ankurah/ankql');

    const planner = new Planner(plannerConfigIndexeddb());
    const selection = parseSelection("licensing.territory = 'US'");
    const plans = planner.plan(selection, 'id');

    // Find the index plan
    const indexPlan = plans.find((p: any) => p.is('Index'));
    expect(indexPlan).toBeDefined();

    indexPlan.match({
      Index: (data: any) => {
        // Verify keypart has sub_path
        expect(data.indexSpec.keyparts.length).toBeGreaterThanOrEqual(1);
        const keypart = data.indexSpec.keyparts[0];
        expect(keypart.column).toBe('licensing');
        expect(keypart.subPath).toEqual(['territory']);

        // Verify full pushdown (remaining predicate should be True)
        expect(data.remainingPredicate.is('True')).toBe(true);
      },
      TableScan: () => { throw new Error('unexpected'); },
      EmptyScan: () => { throw new Error('unexpected'); },
    });
  });
});
