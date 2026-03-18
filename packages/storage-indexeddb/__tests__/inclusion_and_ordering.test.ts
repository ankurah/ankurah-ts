// MIRRORS: ankurah/storage/indexeddb-wasm/tests/inclusion_and_ordering.rs

import { describe, test, expect } from 'bun:test';
import {
  createIndexedDBNode, createAlbums, createEvents, Album, Event,
  names, sortNames, years, eventTimestamps, matchArgs, IndexedDBStorageEngine,
} from './common.ts';

describe('inclusion_and_ordering', () => {
  test('test_comprehensive_set_inclusion_and_ordering', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    // Create test data with duplicates and edge cases
    await createAlbums(ctx, [
      ['Abbey Road', '1969'],
      ['Revolver', '1966'],
      ['Sgt. Pepper', '1967'],
      ['The White Album', '1968'],
      ['Let It Be', '1970'],
      ['Abbey Road Deluxe', '1969'], // Duplicate year
      ['Help!', '1965'],
    ]);

    // DESC ordering (reverse scan direction)
    expect(
      years(await ctx.fetch(Album, matchArgs("year >= '1965' ORDER BY year DESC"))),
    ).toEqual(['1970', '1969', '1969', '1968', '1967', '1966', '1965']);

    // single operators
    expect(sortNames(await ctx.fetch(Album, matchArgs("year = '1969'")))).toEqual(['Abbey Road', 'Abbey Road Deluxe']);
    expect(years(await ctx.fetch(Album, matchArgs("year < '1968'")))).toEqual(['1965', '1966', '1967']);
    expect(years(await ctx.fetch(Album, matchArgs("year <= '1968'")))).toEqual(['1965', '1966', '1967', '1968']);
    expect(names(await ctx.fetch(Album, matchArgs("year = '1999'")))).toEqual([]);
    expect(names(await ctx.fetch(Album, matchArgs("name = 'Help!'")))).toEqual(['Help!']);

    // Complex range with ordering
    expect(
      names(await ctx.fetch(Album, matchArgs("year >= '1967' AND year <= '1969' ORDER BY name"))),
    ).toEqual(['Abbey Road', 'Abbey Road Deluxe', 'Sgt. Pepper', 'The White Album']);

    // DESC ordering by name
    expect(
      names(await ctx.fetch(Album, matchArgs("year >= '1965' ORDER BY name DESC"))),
    ).toEqual(['The White Album', 'Sgt. Pepper', 'Revolver', 'Let It Be', 'Help!', 'Abbey Road Deluxe', 'Abbey Road']);

    // LIMIT with DESC ordering
    expect(
      years(await ctx.fetch(Album, matchArgs("year >= '1965' ORDER BY year DESC LIMIT 3"))),
    ).toEqual(['1970', '1969', '1969']);

    // Set exclusion validation
    expect(
      sortNames(await ctx.fetch(Album, matchArgs("year >= '1968'"))),
    ).toEqual(['Abbey Road', 'Abbey Road Deluxe', 'Let It Be', 'The White Album']);

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_room_filter_desc_limit', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const baseTs = 1_762_643_440_000;
    const events: [string, number, boolean][] = [];

    // Primary room events (36 items)
    for (let i = 0; i < 36; i++) {
      events.push(['chat-main', baseTs + i, true]);
    }

    // Some events that should be filtered out
    for (let i = 0; i < 5; i++) {
      events.push(['chat-main', baseTs + 1_000 + i, false]);
    }

    // Other room noise
    for (let i = 0; i < 10; i++) {
      events.push(['chat-other', baseTs + i, true]);
    }

    await createEvents(ctx, events);

    // Test ASC ordering first
    const resultsAsc = await ctx.fetch(Event, matchArgs("name = 'chat-main' AND active = true ORDER BY timestamp ASC LIMIT 20"));
    expect(resultsAsc.length).toBe(20);
    const tsAsc = eventTimestamps(resultsAsc);
    expect(tsAsc.every((v, i) => i === 0 || tsAsc[i - 1] < v)).toBe(true);

    // Test DESC ordering
    const resultsDesc = await ctx.fetch(Event, matchArgs("name = 'chat-main' AND active = true ORDER BY timestamp DESC LIMIT 20"));
    expect(resultsDesc.length).toBe(20);
    const tsDesc = eventTimestamps(resultsDesc);
    expect(tsDesc.every((v, i) => i === 0 || tsDesc[i - 1] > v)).toBe(true);

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_i64_bool_indexing', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createEvents(ctx, [
      ['Event0', 100, true],
      ['Event1', 200, false],
      ['Event2', 300, true],
      ['Event3', 400, false],
      ['Event4', 500, true],
      ['Event5', 600, false],
    ]);

    // Test 1: Range query on i64 timestamp
    const results1 = await ctx.fetch(Event, matchArgs('timestamp > 350'));
    expect(results1.length).toBe(3);

    // Test 2: Query by bool field (equality)
    const activeResults = await ctx.fetch(Event, matchArgs('active = true'));
    expect(activeResults.length).toBe(3);

    const inactiveResults = await ctx.fetch(Event, matchArgs('active = false'));
    expect(inactiveResults.length).toBe(3);

    // Test 3: Compound query (bool AND i64 range)
    const results3 = await ctx.fetch(Event, matchArgs('active = true AND timestamp >= 200'));
    expect(results3.length).toBe(2);

    // Test 4: ORDER BY timestamp DESC
    const results4 = await ctx.fetch(Event, matchArgs('timestamp > 0 ORDER BY timestamp DESC LIMIT 3'));
    expect(results4.length).toBe(3);
    expect(results4[0].timestamp()).toBeGreaterThan(results4[1].timestamp());
    expect(results4[1].timestamp()).toBeGreaterThan(results4[2].timestamp());

    // Test 5: Disjunction with boolean forces residual predicate evaluation
    const results5 = await ctx.fetch(Event, matchArgs("timestamp > 200 AND (active = true OR name = 'Event0')"));
    expect(results5.length).toBe(2);

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_large_i64_timestamp', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    // Use timestamps around MAX_SAFE_INTEGER boundary
    await createEvents(ctx, [
      ['Event1', 9_007_199_254_740_990, true],  // Just below threshold (number)
      ['Event2', 9_007_199_254_740_991, false], // At threshold (number)
      ['Event3', 9_007_199_254_740_992, true],  // Beyond threshold (string)
      ['Event4', 9_007_199_254_741_000, false], // Beyond threshold (string)
    ]);

    // Range query spanning the number/string threshold
    const results = await ctx.fetch(Event, matchArgs('timestamp > 9007199254740990'));
    expect(results.length).toBe(3);

    // Verify ordering is maintained across threshold
    const allResults = await ctx.fetch(Event, matchArgs('timestamp > 0 ORDER BY timestamp ASC'));
    expect(allResults.length).toBe(4);
    for (let i = 0; i < 3; i++) {
      expect(allResults[i].timestamp()).toBeLessThan(allResults[i + 1].timestamp());
    }

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_equality_prefix_edge_cases', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    // Test 1: Single equality column DESC
    await createEvents(ctx, [
      ['alpha', 100, true], ['alpha', 200, true], ['alpha', 300, true], ['bravo', 100, true],
    ]);

    const results1 = await ctx.fetch(Event, matchArgs("name = 'alpha' ORDER BY timestamp DESC LIMIT 2"));
    expect(results1.length).toBe(2);
    expect(eventTimestamps(results1)).toEqual([300, 200]);

    // Test 2: Bool boundary
    await createEvents(ctx, [
      ['charlie', 1000, false], ['charlie', 2000, true], ['charlie', 3000, true],
    ]);

    const trueResults = await ctx.fetch(Event, matchArgs("name = 'charlie' AND active = true ORDER BY timestamp DESC"));
    expect(trueResults.length).toBe(2);
    expect(eventTimestamps(trueResults)).toEqual([3000, 2000]);

    const falseResults = await ctx.fetch(Event, matchArgs("name = 'charlie' AND active = false ORDER BY timestamp DESC"));
    expect(falseResults.length).toBe(1);
    expect(eventTimestamps(falseResults)).toEqual([1000]);

    // Test 3: Negative timestamps
    await createEvents(ctx, [
      ['delta', -300, true], ['delta', -200, true], ['delta', -100, true], ['delta', 0, true], ['delta', 100, true],
    ]);

    const negResults = await ctx.fetch(Event, matchArgs("name = 'delta' AND active = true ORDER BY timestamp DESC LIMIT 3"));
    expect(negResults.length).toBe(3);
    expect(eventTimestamps(negResults)).toEqual([100, 0, -100]);

    const negAsc = await ctx.fetch(Event, matchArgs("name = 'delta' AND active = true ORDER BY timestamp ASC LIMIT 3"));
    expect(eventTimestamps(negAsc)).toEqual([-300, -200, -100]);

    // Test 4: Zero values
    await createEvents(ctx, [
      ['echo', 0, false], ['echo', 0, true], ['echo', 1, false], ['echo', 1, true],
    ]);

    const zeroFalse = await ctx.fetch(Event, matchArgs("name = 'echo' AND timestamp = 0 AND active = false"));
    expect(zeroFalse.length).toBe(1);

    const zeroTrue = await ctx.fetch(Event, matchArgs("name = 'echo' AND timestamp = 0 AND active = true"));
    expect(zeroTrue.length).toBe(1);

    // Test 5: Empty result set with bounded range
    const empty = await ctx.fetch(Event, matchArgs("name = 'foxtrot' AND active = true ORDER BY timestamp DESC LIMIT 10"));
    expect(empty.length).toBe(0);

    // Cleanup
    await IndexedDBStorageEngine.cleanup(dbName);
  });
});
