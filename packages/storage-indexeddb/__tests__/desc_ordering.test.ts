// MIRRORS: ankurah/storage/indexeddb-wasm/tests/desc_ordering.rs

import { describe, test, expect } from 'bun:test';
import {
  createIndexedDBNode, LogEvent, Message,
  eventTimestamps, assertDescOrder, assertAscOrder,
  matchArgs, IndexedDBStorageEngine,
} from './common.ts';

const TIMESTAMP_BASE = 1700000000000;
const TIMESTAMP_STEP = 1000;

function msgTimestamps(msgs: any[]): number[] { return msgs.map((m: any) => m.timestamp()); }

async function createLogEvents(ctx: any, events: [string, number, string][]): Promise<void> {
  const trx = ctx.begin();
  for (const [category, timestamp, level] of events) {
    await trx.create(LogEvent, { category, timestamp, level });
  }
  await trx.commit();
}

async function createMessages(ctx: any, messages: [string, boolean, number, string][]): Promise<void> {
  const trx = ctx.begin();
  for (const [room, deleted, timestamp, text] of messages) {
    await trx.create(Message, { room, deleted, timestamp, text });
  }
  await trx.commit();
}

describe('desc_ordering', () => {
  // Section 1: Basic DESC Ordering with Inequality (No Equality Prefix)
  test('test_desc_inequality_no_equality_prefix', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const events: [string, number, string][] = Array.from({ length: 10 }, (_, i) => [
      'logs', TIMESTAMP_BASE + i * TIMESTAMP_STEP, i % 2 === 0 ? 'INFO' : 'ERROR',
    ]);
    await createLogEvents(ctx, events);

    const tsMid = TIMESTAMP_BASE + 5 * TIMESTAMP_STEP;
    const tsMax = TIMESTAMP_BASE + 9 * TIMESTAMP_STEP;

    // Test <=
    const r1 = await ctx.fetch(LogEvent, matchArgs(`timestamp <= ${tsMid} ORDER BY timestamp DESC`));
    const ts1 = eventTimestamps(r1);
    expect(r1.length).toBe(6);
    assertDescOrder(ts1, 'Should be DESC order');
    expect(ts1[0]).toBe(tsMid);

    // Test >=
    const r2 = await ctx.fetch(LogEvent, matchArgs(`timestamp >= ${tsMid} ORDER BY timestamp DESC`));
    const ts2 = eventTimestamps(r2);
    expect(r2.length).toBe(5);
    expect(ts2[0]).toBe(tsMax);
    expect(ts2[ts2.length - 1]).toBe(tsMid);

    // Test <
    const r3 = await ctx.fetch(LogEvent, matchArgs(`timestamp < ${tsMid} ORDER BY timestamp DESC`));
    expect(r3.length).toBe(5);

    // Test >
    const r4 = await ctx.fetch(LogEvent, matchArgs(`timestamp > ${tsMid} ORDER BY timestamp DESC`));
    expect(r4.length).toBe(4);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Section 2: Single Equality Prefix + DESC Inequality
  test('test_desc_inequality_single_equality_prefix', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const events: [string, number, string][] = [];
    for (let i = 0; i < 10; i++) {
      events.push(['cat_a', TIMESTAMP_BASE + i * TIMESTAMP_STEP, 'INFO']);
      events.push(['cat_b', TIMESTAMP_BASE + i * TIMESTAMP_STEP, 'ERROR']);
    }
    await createLogEvents(ctx, events);

    const tsMid = TIMESTAMP_BASE + 5 * TIMESTAMP_STEP;

    // Test: category = 'cat_a' AND timestamp <= mid ORDER BY timestamp DESC
    const r1 = await ctx.fetch(LogEvent, matchArgs(`category = 'cat_a' AND timestamp <= ${tsMid} ORDER BY timestamp DESC`));
    const ts1 = eventTimestamps(r1);
    expect(r1.length).toBe(6);
    assertDescOrder(ts1, 'Should be DESC order');
    expect(ts1[0]).toBe(tsMid);

    // Verify all results are from cat_a
    for (const event of r1) {
      expect(event.category()).toBe('cat_a');
    }

    // Test: category = 'cat_b' AND timestamp >= mid ORDER BY timestamp DESC
    const r2 = await ctx.fetch(LogEvent, matchArgs(`category = 'cat_b' AND timestamp >= ${tsMid} ORDER BY timestamp DESC`));
    const ts2 = eventTimestamps(r2);
    expect(r2.length).toBe(5);
    assertDescOrder(ts2, 'Should be DESC order');

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Section 3: Two Equality Columns + DESC Inequality (Chat Message Pattern)
  test('test_desc_inequality_two_equality_prefix_lte', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const messages: [string, boolean, number, string][] = [];
    for (let i = 0; i < 10; i++) {
      messages.push(['room_1', false, TIMESTAMP_BASE + i * TIMESTAMP_STEP, `Message ${i}`]);
    }
    messages.push(['room_1', true, TIMESTAMP_BASE + 5 * TIMESTAMP_STEP, 'Deleted']);
    messages.push(['room_2', false, TIMESTAMP_BASE + 5 * TIMESTAMP_STEP, 'Other room']);
    await createMessages(ctx, messages);

    const tsMid = TIMESTAMP_BASE + 5 * TIMESTAMP_STEP;

    const r = await ctx.fetch(Message, matchArgs(`room = 'room_1' AND deleted = false AND timestamp <= ${tsMid} ORDER BY timestamp DESC`));
    const ts = msgTimestamps(r);
    expect(r.length).toBe(6);
    assertDescOrder(ts, 'Should be DESC order');
    expect(ts[0]).toBe(tsMid);

    for (const msg of r) {
      expect(msg.room()).toBe('room_1');
      expect(msg.deleted()).toBe(false);
    }

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_desc_inequality_two_equality_prefix_gte', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const messages: [string, boolean, number, string][] = [];
    for (let i = 0; i < 10; i++) {
      messages.push(['room_1', false, TIMESTAMP_BASE + i * TIMESTAMP_STEP, `Message ${i}`]);
    }
    messages.push(['room_1', true, TIMESTAMP_BASE + 5 * TIMESTAMP_STEP, 'Deleted']);
    messages.push(['room_2', false, TIMESTAMP_BASE + 5 * TIMESTAMP_STEP, 'Other room']);
    await createMessages(ctx, messages);

    const tsMid = TIMESTAMP_BASE + 5 * TIMESTAMP_STEP;
    const tsMax = TIMESTAMP_BASE + 9 * TIMESTAMP_STEP;

    const r = await ctx.fetch(Message, matchArgs(`room = 'room_1' AND deleted = false AND timestamp >= ${tsMid} ORDER BY timestamp DESC`));
    const ts = msgTimestamps(r);
    expect(r.length).toBe(5);
    expect(ts[0]).toBe(tsMax);
    expect(ts[ts.length - 1]).toBe(tsMid);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Section 4: Range Queries with DESC Ordering
  test('test_range_inclusive_inclusive_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const events: [string, number, string][] = Array.from({ length: 10 }, (_, i) => [
      'logs', TIMESTAMP_BASE + i * TIMESTAMP_STEP, 'INFO',
    ]);
    await createLogEvents(ctx, events);

    const ts3 = TIMESTAMP_BASE + 3 * TIMESTAMP_STEP;
    const ts7 = TIMESTAMP_BASE + 7 * TIMESTAMP_STEP;

    const r = await ctx.fetch(LogEvent, matchArgs(`timestamp >= ${ts3} AND timestamp <= ${ts7} ORDER BY timestamp DESC`));
    const ts = eventTimestamps(r);
    expect(r.length).toBe(5);
    expect(ts).toContain(ts3);
    expect(ts).toContain(ts7);
    expect(ts[0]).toBe(ts7);
    expect(ts[ts.length - 1]).toBe(ts3);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_range_exclusive_exclusive_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const events: [string, number, string][] = Array.from({ length: 10 }, (_, i) => [
      'logs', TIMESTAMP_BASE + i * TIMESTAMP_STEP, 'INFO',
    ]);
    await createLogEvents(ctx, events);

    const ts3 = TIMESTAMP_BASE + 3 * TIMESTAMP_STEP;
    const ts7 = TIMESTAMP_BASE + 7 * TIMESTAMP_STEP;

    const r = await ctx.fetch(LogEvent, matchArgs(`timestamp > ${ts3} AND timestamp < ${ts7} ORDER BY timestamp DESC`));
    const ts = eventTimestamps(r);
    expect(r.length).toBe(3);
    expect(ts).not.toContain(ts3);
    expect(ts).not.toContain(ts7);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Section 5: LIMIT with DESC Ordering
  test('test_limit_with_desc_inequality', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const events: [string, number, string][] = Array.from({ length: 20 }, (_, i) => [
      'logs', TIMESTAMP_BASE + i * TIMESTAMP_STEP, 'INFO',
    ]);
    await createLogEvents(ctx, events);

    const tsMid = TIMESTAMP_BASE + 15 * TIMESTAMP_STEP;

    const r = await ctx.fetch(LogEvent, matchArgs(`timestamp <= ${tsMid} ORDER BY timestamp DESC LIMIT 5`));
    const ts = eventTimestamps(r);
    expect(r.length).toBe(5);
    assertDescOrder(ts, 'Should be DESC order');
    expect(ts[0]).toBe(tsMid);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_limit_with_equality_prefix_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const messages: [string, boolean, number, string][] = [];
    for (let i = 0; i < 50; i++) {
      messages.push(['room_1', false, TIMESTAMP_BASE + i * TIMESTAMP_STEP, `Msg ${i}`]);
    }
    await createMessages(ctx, messages);

    const tsBoundary = TIMESTAMP_BASE + 40 * TIMESTAMP_STEP;

    const r = await ctx.fetch(Message, matchArgs(`room = 'room_1' AND deleted = false AND timestamp <= ${tsBoundary} ORDER BY timestamp DESC LIMIT 20`));
    const ts = msgTimestamps(r);
    expect(r.length).toBe(20);
    assertDescOrder(ts, 'Should be DESC order');
    expect(ts[0]).toBe(tsBoundary);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Section 6: Edge Cases
  test('test_empty_result_set_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const events: [string, number, string][] = Array.from({ length: 10 }, (_, i) => [
      'logs', TIMESTAMP_BASE + i * TIMESTAMP_STEP, 'INFO',
    ]);
    await createLogEvents(ctx, events);

    const r = await ctx.fetch(LogEvent, matchArgs(`timestamp < ${TIMESTAMP_BASE - 1000} ORDER BY timestamp DESC`));
    expect(r.length).toBe(0);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_single_result_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const events: [string, number, string][] = Array.from({ length: 10 }, (_, i) => [
      'logs', TIMESTAMP_BASE + i * TIMESTAMP_STEP, 'INFO',
    ]);
    await createLogEvents(ctx, events);

    const ts5 = TIMESTAMP_BASE + 5 * TIMESTAMP_STEP;

    const r = await ctx.fetch(LogEvent, matchArgs(`timestamp >= ${ts5} AND timestamp <= ${ts5} ORDER BY timestamp DESC`));
    expect(r.length).toBe(1);
    expect(r[0].timestamp()).toBe(ts5);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_duplicate_timestamps_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const sameTs = TIMESTAMP_BASE + 5000;
    const events: [string, number, string][] = Array.from({ length: 5 }, (_, i) => [
      'logs', sameTs, i % 2 === 0 ? 'INFO' : 'ERROR',
    ]);
    await createLogEvents(ctx, events);

    const r1 = await ctx.fetch(LogEvent, matchArgs(`timestamp <= ${sameTs} ORDER BY timestamp DESC`));
    expect(r1.length).toBe(5);

    const r2 = await ctx.fetch(LogEvent, matchArgs(`timestamp < ${sameTs} ORDER BY timestamp DESC`));
    expect(r2.length).toBe(0);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Section 7: ASC Ordering Sanity Check
  test('test_asc_ordering_with_inequality', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    const events: [string, number, string][] = Array.from({ length: 10 }, (_, i) => [
      'logs', TIMESTAMP_BASE + i * TIMESTAMP_STEP, 'INFO',
    ]);
    await createLogEvents(ctx, events);

    const tsMid = TIMESTAMP_BASE + 5 * TIMESTAMP_STEP;

    const r = await ctx.fetch(LogEvent, matchArgs(`timestamp <= ${tsMid} ORDER BY timestamp ASC`));
    const ts = eventTimestamps(r);
    expect(r.length).toBe(6);
    assertAscOrder(ts, 'Should be ASC order');
    expect(ts[0]).toBe(TIMESTAMP_BASE);
    expect(ts[ts.length - 1]).toBe(tsMid);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Section 8: Pagination Pattern (Real-World Scenario)
  test('test_pagination_pattern_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    // Create 100 messages
    const messages: [string, boolean, number, string][] = [];
    for (let i = 0; i < 100; i++) {
      messages.push(['room_1', false, TIMESTAMP_BASE + i * TIMESTAMP_STEP, `Msg ${i}`]);
    }
    await createMessages(ctx, messages);

    // Initial load: get 33 newest messages
    const initial = await ctx.fetch(Message, matchArgs("room = 'room_1' AND deleted = false ORDER BY timestamp DESC LIMIT 33"));
    expect(initial.length).toBe(33);
    const initialTs = msgTimestamps(initial);
    assertDescOrder(initialTs, 'Initial load');

    const newestTs = initialTs[0];
    const oldestInPage = initialTs[initialTs.length - 1];

    expect(newestTs).toBe(TIMESTAMP_BASE + 99 * TIMESTAMP_STEP);
    expect(oldestInPage).toBe(TIMESTAMP_BASE + 67 * TIMESTAMP_STEP);

    // Pagination: expand window to get 54 total messages <= newest
    const expanded = await ctx.fetch(Message, matchArgs(`room = 'room_1' AND deleted = false AND timestamp <= ${newestTs} ORDER BY timestamp DESC LIMIT 54`));
    expect(expanded.length).toBe(54);
    const expandedTs = msgTimestamps(expanded);
    assertDescOrder(expandedTs, 'Expanded load');
    expect(expandedTs[0]).toBe(newestTs);

    await IndexedDBStorageEngine.cleanup(dbName);
  });
});
