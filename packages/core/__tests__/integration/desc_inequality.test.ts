// MIRRORS: ankurah/tests/tests/desc_inequality.rs
//
// Core tests for DESC inequality bounds in composite indexes.
//
// This test suite covers the bug fix in PR #212 where the query planner was incorrectly
// checking if the FIRST column of an index was DESC, instead of checking if the
// INEQUALITY column was DESC.
//
// ## The Bug (PR #212)
//
// For a composite index like [room ASC, deleted ASC, timestamp DESC]:
// - Old code checked: keySpec.keyparts.first().direction.isDesc() -> FALSE (room is ASC)
// - Fixed code checks: keySpec.keyparts.get(eqPrefixLen).direction.isDesc() -> TRUE

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import type { EntityId } from '@ankurah/proto';
import { Node, matchArgs } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, lww, yrsText } from '../../src/define-model.ts';

// ============================================================================
// Test Models
// ============================================================================

// Mirrors: desc_inequality.rs TestMessage
// Composite index: [room ASC, deleted ASC, timestamp DESC]
// Divergence: Rust `Ref<TestRoom>` is an EntityId stored as LWW; TS uses lww<string>() for the base64 ID [E1].
// Divergence: Rust `#[active_type(LWW)] bool` maps to lww<boolean>() [E1].
const TestMessage = defineModel('testmessage', {
  room: lww<string>(),
  text: yrsText(),
  timestamp: lww<number>(),
  deleted: lww<boolean>(),
});

// Mirrors: desc_inequality.rs TestRoom
const TestRoom = defineModel('testroom', {
  name: yrsText(),
});

// Mirrors: desc_inequality.rs SimpleEvent
// Index: [timestamp DESC]
const SimpleEvent = defineModel('simpleevent', {
  timestamp: lww<number>(),
  data: yrsText(),
});

// Mirrors: desc_inequality.rs ScoredItem
// Index: [category ASC, score DESC]
const ScoredItem = defineModel('scoreditem', {
  category: lww<string>(),
  score: lww<number>(),
  name: yrsText(),
});

// Mirrors: desc_inequality.rs Task
// Index: [org ASC, team ASC, project ASC, priority DESC]
const Task = defineModel('task', {
  org: lww<string>(),
  team: lww<string>(),
  project: lww<string>(),
  priority: lww<number>(),
  title: yrsText(),
});

// ============================================================================
// Test Constants
// ============================================================================

const TIMESTAMP_BASE = 1700000000000;
const TIMESTAMP_STEP = 1000;

// ============================================================================
// Helper Functions
// ============================================================================

function createTestNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
}

// Mirrors: desc_inequality.rs setup()
function setup(): { ctx: ReturnType<Node['context']> } {
  const node = createTestNode();
  const ctx = node.context();
  return { ctx };
}

// Mirrors: desc_inequality.rs create_messages()
async function createMessages(
  ctx: ReturnType<Node['context']>,
  roomId: EntityId,
  count: number,
): Promise<number[]> {
  const trx = ctx.begin();
  const timestamps: number[] = [];
  for (let i = 0; i < count; i++) {
    const ts = TIMESTAMP_BASE + i * TIMESTAMP_STEP;
    timestamps.push(ts);
    await trx.create(TestMessage, {
      room: roomId.toBase64(),
      text: `Message #${String(i).padStart(3, '0')}`,
      timestamp: ts,
      deleted: false,
    });
  }
  await trx.commit();
  return timestamps;
}

// Mirrors: desc_inequality.rs create_room()
async function createRoom(ctx: ReturnType<Node['context']>, name: string): Promise<EntityId> {
  const trx = ctx.begin();
  const room = await trx.create(TestRoom, { name });
  const id = room.inner.id();
  await trx.commit();
  return id;
}

// Mirrors: desc_inequality.rs get_timestamps()
function getTimestamps(results: Array<{ timestamp(): number }>): number[] {
  return results.map((m) => m.timestamp());
}

// Mirrors: desc_inequality.rs assert_desc_order()
function assertDescOrder(timestamps: number[], context: string): void {
  for (let i = 0; i < timestamps.length - 1; i++) {
    expect(timestamps[i]).toBeGreaterThanOrEqual(timestamps[i + 1]);
  }
}

// Mirrors: desc_inequality.rs assert_asc_order()
function assertAscOrder(timestamps: number[], context: string): void {
  for (let i = 0; i < timestamps.length - 1; i++) {
    expect(timestamps[i]).toBeLessThanOrEqual(timestamps[i + 1]);
  }
}

// ============================================================================
// SECTION 1: Equality Prefix Variations (eq_prefix_len = 0, 1, 2, 3)
// ============================================================================

describe('desc_inequality', () => {
  // Mirrors: desc_inequality.rs test_desc_inequality_no_equality_prefix
  test('test_desc_inequality_no_equality_prefix', async () => {
    const { ctx } = setup();

    {
      const trx = ctx.begin();
      for (let i = 0; i < 10; i++) {
        const ts = TIMESTAMP_BASE + i * TIMESTAMP_STEP;
        await trx.create(SimpleEvent, { timestamp: ts, data: `Event ${i}` });
      }
      await trx.commit();
    }

    const tsMid = TIMESTAMP_BASE + 5 * TIMESTAMP_STEP;
    const tsMax = TIMESTAMP_BASE + 9 * TIMESTAMP_STEP;

    // Test <=
    const q1 = `timestamp <= ${tsMid} ORDER BY timestamp DESC`;
    const results1 = await ctx.fetch(SimpleEvent, matchArgs(q1));
    const timestamps1 = getTimestamps(results1);

    expect(results1.length).toBe(6);
    assertDescOrder(timestamps1, 'timestamp <= mid');
    expect(timestamps1[0]).toBe(tsMid);

    // Test >=
    const q2 = `timestamp >= ${tsMid} ORDER BY timestamp DESC`;
    const results2 = await ctx.fetch(SimpleEvent, matchArgs(q2));
    const timestamps2 = getTimestamps(results2);

    expect(results2.length).toBe(5);
    expect(timestamps2[0]).toBe(tsMax);
    expect(timestamps2[timestamps2.length - 1]).toBe(tsMid);

    // Test <
    const q3 = `timestamp < ${tsMid} ORDER BY timestamp DESC`;
    const results3 = await ctx.fetch(SimpleEvent, matchArgs(q3));
    expect(results3.length).toBe(5);

    // Test >
    const q4 = `timestamp > ${tsMid} ORDER BY timestamp DESC`;
    const results4 = await ctx.fetch(SimpleEvent, matchArgs(q4));
    expect(results4.length).toBe(4);
  });

  // Mirrors: desc_inequality.rs test_desc_inequality_single_equality_prefix
  test('test_desc_inequality_single_equality_prefix', async () => {
    const { ctx } = setup();

    {
      const trx = ctx.begin();
      for (let i = 0; i < 10; i++) {
        await trx.create(ScoredItem, { category: 'A', score: i * 10, name: `Item A-${i}` });
        await trx.create(ScoredItem, { category: 'B', score: i * 10, name: `Item B-${i}` });
      }
      await trx.commit();
    }

    const scoreMid = 50;

    // Test: category = 'A' AND score <= 50 ORDER BY score DESC
    const q1 = `category = 'A' AND score <= ${scoreMid} ORDER BY score DESC`;
    const results1 = await ctx.fetch(ScoredItem, matchArgs(q1));
    const scores1: number[] = results1.map((s) => s.score());

    expect(results1.length).toBe(6);
    expect(scores1).toEqual([50, 40, 30, 20, 10, 0]);

    for (const item of results1) {
      expect(item.category()).toBe('A');
    }

    // Test: category = 'B' AND score >= 50 ORDER BY score DESC
    const q2 = `category = 'B' AND score >= ${scoreMid} ORDER BY score DESC`;
    const results2 = await ctx.fetch(ScoredItem, matchArgs(q2));
    const scores2: number[] = results2.map((s) => s.score());

    expect(results2.length).toBe(5);
    expect(scores2).toEqual([90, 80, 70, 60, 50]);
  });

  // Mirrors: desc_inequality.rs test_desc_inequality_two_equality_prefix
  // THE canonical PR #212 bug scenario
  test('test_desc_inequality_two_equality_prefix', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const tsMid = timestamps[5];
    const tsMax = timestamps[9];

    // Test <=
    const q1 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${tsMid} ORDER BY timestamp DESC`;
    const results1 = await ctx.fetch(TestMessage, matchArgs(q1));
    const resultTs1 = getTimestamps(results1);

    expect(results1.length).toBe(6);
    assertDescOrder(resultTs1, 'timestamp <= mid');
    expect(resultTs1[0]).toBe(tsMid);
    expect(resultTs1[resultTs1.length - 1]).toBe(timestamps[0]);

    // Test >=
    const q2 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp >= ${tsMid} ORDER BY timestamp DESC`;
    const results2 = await ctx.fetch(TestMessage, matchArgs(q2));
    const resultTs2 = getTimestamps(results2);

    expect(results2.length).toBe(5);
    expect(resultTs2[0]).toBe(tsMax);
    expect(resultTs2[resultTs2.length - 1]).toBe(tsMid);
  });

  // Mirrors: desc_inequality.rs test_desc_inequality_three_equality_prefix
  test('test_desc_inequality_three_equality_prefix', async () => {
    const { ctx } = setup();

    {
      const trx = ctx.begin();
      for (let i = 0; i < 10; i++) {
        await trx.create(Task, {
          org: 'Acme',
          team: 'Engineering',
          project: 'Backend',
          priority: i * 10,
          title: `Task ${i}`,
        });
      }
      await trx.commit();
    }

    const priorityMid = 50;

    const q = `org = 'Acme' AND team = 'Engineering' AND project = 'Backend' AND priority <= ${priorityMid} ORDER BY priority DESC`;
    const results = await ctx.fetch(Task, matchArgs(q));
    const priorities: number[] = results.map((t) => t.priority());

    expect(results.length).toBe(6);
    expect(priorities).toEqual([50, 40, 30, 20, 10, 0]);
  });

  // ============================================================================
  // SECTION 2: Strict Inequality Operators (< and >)
  // ============================================================================

  // Mirrors: desc_inequality.rs test_operator_less_than_desc
  test('test_operator_less_than_desc', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const ts5 = timestamps[5];

    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp < ${ts5} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));
    const resultTs = getTimestamps(results);

    expect(results.length).toBe(5);
    expect(resultTs.includes(ts5)).toBe(false);
    expect(resultTs[0]).toBe(timestamps[4]);
  });

  // Mirrors: desc_inequality.rs test_operator_greater_than_desc
  test('test_operator_greater_than_desc', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const ts5 = timestamps[5];

    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp > ${ts5} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));
    const resultTs = getTimestamps(results);

    expect(results.length).toBe(4);
    expect(resultTs.includes(ts5)).toBe(false);
    expect(resultTs[resultTs.length - 1]).toBe(timestamps[6]);
  });

  // ============================================================================
  // SECTION 3: Range Queries (All Inclusivity Combinations)
  // ============================================================================

  // Mirrors: desc_inequality.rs test_range_inclusive_inclusive
  test('test_range_inclusive_inclusive', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const ts3 = timestamps[3];
    const ts7 = timestamps[7];

    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp >= ${ts3} AND timestamp <= ${ts7} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));
    const resultTs = getTimestamps(results);

    expect(results.length).toBe(5);
    expect(resultTs).toEqual([ts7, timestamps[6], timestamps[5], timestamps[4], ts3]);
    expect(resultTs.includes(ts3)).toBe(true);
    expect(resultTs.includes(ts7)).toBe(true);
  });

  // Mirrors: desc_inequality.rs test_range_exclusive_exclusive
  test('test_range_exclusive_exclusive', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const ts3 = timestamps[3];
    const ts7 = timestamps[7];

    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp > ${ts3} AND timestamp < ${ts7} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));
    const resultTs = getTimestamps(results);

    expect(results.length).toBe(3);
    expect(resultTs.includes(ts3)).toBe(false);
    expect(resultTs.includes(ts7)).toBe(false);
    expect(resultTs).toEqual([timestamps[6], timestamps[5], timestamps[4]]);
  });

  // Mirrors: desc_inequality.rs test_range_inclusive_exclusive
  test('test_range_inclusive_exclusive', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const ts3 = timestamps[3];
    const ts7 = timestamps[7];

    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp >= ${ts3} AND timestamp < ${ts7} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));
    const resultTs = getTimestamps(results);

    expect(results.length).toBe(4);
    expect(resultTs.includes(ts3)).toBe(true);
    expect(resultTs.includes(ts7)).toBe(false);
  });

  // Mirrors: desc_inequality.rs test_range_exclusive_inclusive
  test('test_range_exclusive_inclusive', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const ts3 = timestamps[3];
    const ts7 = timestamps[7];

    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp > ${ts3} AND timestamp <= ${ts7} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));
    const resultTs = getTimestamps(results);

    expect(results.length).toBe(4);
    expect(resultTs.includes(ts3)).toBe(false);
    expect(resultTs.includes(ts7)).toBe(true);
  });

  // ============================================================================
  // SECTION 4: Boundary Conditions & Edge Cases
  // ============================================================================

  // Mirrors: desc_inequality.rs test_empty_result_set
  test('test_empty_result_set', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    await createMessages(ctx, roomId, 10);

    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp < ${TIMESTAMP_BASE - 1000} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));

    expect(results.length).toBe(0);
  });

  // Mirrors: desc_inequality.rs test_single_result
  test('test_single_result', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const ts5 = timestamps[5];
    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp >= ${ts5} AND timestamp <= ${ts5} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));

    expect(results.length).toBe(1);
    expect(results[0].timestamp()).toBe(ts5);
  });

  // Mirrors: desc_inequality.rs test_duplicate_timestamps
  test('test_duplicate_timestamps', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');

    const sameTs = TIMESTAMP_BASE + 5000;
    {
      const trx = ctx.begin();
      for (let i = 0; i < 5; i++) {
        await trx.create(TestMessage, {
          room: roomId.toBase64(),
          text: `Duplicate ${i}`,
          timestamp: sameTs,
          deleted: false,
        });
      }
      await trx.commit();
    }

    // Query <= sameTs should get all 5
    const q1 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${sameTs} ORDER BY timestamp DESC`;
    const results1 = await ctx.fetch(TestMessage, matchArgs(q1));

    expect(results1.length).toBe(5);
    for (const r of results1) {
      expect(r.timestamp()).toBe(sameTs);
    }

    // Query < sameTs should get none
    const q2 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp < ${sameTs} ORDER BY timestamp DESC`;
    const results2 = await ctx.fetch(TestMessage, matchArgs(q2));
    expect(results2.length).toBe(0);
  });

  // Mirrors: desc_inequality.rs test_boundary_at_minimum
  test('test_boundary_at_minimum', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const tsMin = timestamps[0];

    // <= min should get only the first message
    const q1 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${tsMin} ORDER BY timestamp DESC`;
    const results1 = await ctx.fetch(TestMessage, matchArgs(q1));
    expect(results1.length).toBe(1);
    expect(results1[0].timestamp()).toBe(tsMin);

    // < min should get nothing
    const q2 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp < ${tsMin} ORDER BY timestamp DESC`;
    const results2 = await ctx.fetch(TestMessage, matchArgs(q2));
    expect(results2.length).toBe(0);

    // >= min should get all
    const q3 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp >= ${tsMin} ORDER BY timestamp DESC`;
    const results3 = await ctx.fetch(TestMessage, matchArgs(q3));
    expect(results3.length).toBe(10);
  });

  // Mirrors: desc_inequality.rs test_boundary_at_maximum
  test('test_boundary_at_maximum', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const tsMax = timestamps[9];

    // >= max should get only the last message
    const q1 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp >= ${tsMax} ORDER BY timestamp DESC`;
    const results1 = await ctx.fetch(TestMessage, matchArgs(q1));
    expect(results1.length).toBe(1);
    expect(results1[0].timestamp()).toBe(tsMax);

    // > max should get nothing
    const q2 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp > ${tsMax} ORDER BY timestamp DESC`;
    const results2 = await ctx.fetch(TestMessage, matchArgs(q2));
    expect(results2.length).toBe(0);

    // <= max should get all
    const q3 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${tsMax} ORDER BY timestamp DESC`;
    const results3 = await ctx.fetch(TestMessage, matchArgs(q3));
    expect(results3.length).toBe(10);
  });

  // ============================================================================
  // SECTION 5: ORDER BY Variations
  // ============================================================================

  // Mirrors: desc_inequality.rs test_asc_ordering_not_broken
  test('test_asc_ordering_not_broken', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const tsMid = timestamps[5];

    // ASC with <=
    const q1 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${tsMid} ORDER BY timestamp ASC`;
    const results1 = await ctx.fetch(TestMessage, matchArgs(q1));
    const resultTs1 = getTimestamps(results1);

    expect(results1.length).toBe(6);
    assertAscOrder(resultTs1, 'ASC with <=');
    expect(resultTs1[0]).toBe(timestamps[0]);
    expect(resultTs1[resultTs1.length - 1]).toBe(tsMid);

    // ASC with >=
    const q2 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp >= ${tsMid} ORDER BY timestamp ASC`;
    const results2 = await ctx.fetch(TestMessage, matchArgs(q2));
    const resultTs2 = getTimestamps(results2);

    expect(results2.length).toBe(5);
    assertAscOrder(resultTs2, 'ASC with >=');
    expect(resultTs2[0]).toBe(tsMid);
    expect(resultTs2[resultTs2.length - 1]).toBe(timestamps[9]);
  });

  // Mirrors: desc_inequality.rs test_multi_column_order_by
  test('test_multi_column_order_by', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const tsMid = timestamps[5];

    // Mixed directions: timestamp DESC, text ASC
    const q1 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${tsMid} ORDER BY timestamp DESC, text ASC`;
    const results1 = await ctx.fetch(TestMessage, matchArgs(q1));
    const resultTs1 = getTimestamps(results1);

    expect(results1.length).toBe(6);
    assertDescOrder(resultTs1, 'Primary sort should be DESC (mixed)');

    // Both DESC: timestamp DESC, text DESC
    const q2 = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${tsMid} ORDER BY timestamp DESC, text DESC`;
    const results2 = await ctx.fetch(TestMessage, matchArgs(q2));
    const resultTs2 = getTimestamps(results2);

    expect(results2.length).toBe(6);
    assertDescOrder(resultTs2, 'Primary sort should be DESC (both)');
  });

  // Mirrors: desc_inequality.rs test_no_inequality_just_order_by
  test('test_no_inequality_just_order_by', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 10);

    const q = `room = '${roomId.toBase64()}' AND deleted = false ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));
    const resultTs = getTimestamps(results);

    expect(results.length).toBe(10);
    assertDescOrder(resultTs, 'Should be DESC order');
    expect(resultTs[0]).toBe(timestamps[9]);
    expect(resultTs[resultTs.length - 1]).toBe(timestamps[0]);
  });

  // ============================================================================
  // SECTION 6: Regression Guard
  // ============================================================================

  // Mirrors: desc_inequality.rs test_regression_pr212_desc_inequality_with_asc_prefix
  test('test_regression_pr212_desc_inequality_with_asc_prefix', async () => {
    const { ctx } = setup();
    const roomId = await createRoom(ctx, 'TestRoom');
    const timestamps = await createMessages(ctx, roomId, 50);

    const newestTs = timestamps[49];

    // THE BUG: This query returned only 1 record instead of all 50 because the planner
    // checked if `room` (first column, ASC) was DESC instead of `timestamp` (inequality column, DESC)
    const q = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${newestTs} ORDER BY timestamp DESC`;
    const results = await ctx.fetch(TestMessage, matchArgs(q));

    expect(results.length).toBe(50);

    const resultTs = getTimestamps(results);
    assertDescOrder(resultTs, 'Results should be in DESC order');
    expect(resultTs[0]).toBe(newestTs);
    expect(resultTs[resultTs.length - 1]).toBe(timestamps[0]);
  });
});
