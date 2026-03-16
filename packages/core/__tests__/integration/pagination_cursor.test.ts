// MIRRORS: ankurah/tests/tests/pagination_cursor.rs
//
// End-to-end pagination tests for LiveQuery cursor scenarios.
// These tests verify that pagination works correctly in real-world scenarios
// including local queries, forward pagination, and inter-node communication.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import type { EntityId } from '@ankurah/proto';
import { Node, matchArgs } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, lww, yrsText } from '../../src/define-model.ts';

// ============================================================================
// Test Models
// ============================================================================

// Mirrors: pagination_cursor.rs TestRoom
const TestRoom = defineModel('testroom', {
  name: yrsText(),
});

// Mirrors: pagination_cursor.rs TestMessage
// Divergence: Rust `Ref<TestRoom>` is an EntityId stored as LWW; TS uses lww<string>() for the base64 ID [E1].
// Divergence: Rust `#[active_type(LWW)] bool` maps to lww<boolean>() [E1].
// Divergence: Rust `i64` timestamp maps to lww<number>() [A6].
const TestMessage = defineModel('testmessage', {
  room: lww<string>(),
  text: yrsText(),
  timestamp: lww<number>(),
  deleted: lww<boolean>(),
});

// Mirrors: pagination_cursor.rs ForumPost
const ForumPost = defineModel('forumpost', {
  category: lww<string>(),
  timestamp: lww<number>(),
  author: lww<string>(),
  title: yrsText(),
});

// ============================================================================
// Constants & Helpers
// ============================================================================

const TIMESTAMP_BASE = 1700000000000;
const TIMESTAMP_STEP = 1000;

function createTestNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
}

// Mirrors: pagination_cursor.rs setup()
function setup(): ReturnType<Node['context']> {
  const node = createTestNode();
  return node.context();
}

// Mirrors: pagination_cursor.rs create_room()
async function createRoom(ctx: ReturnType<Node['context']>, name: string): Promise<EntityId> {
  const trx = ctx.begin();
  const room = await trx.create(TestRoom, { name });
  const id = room.inner.id();
  await trx.commit();
  return id;
}

// Mirrors: pagination_cursor.rs get_timestamps()
function getTimestamps(results: Array<{ timestamp(): number }>): number[] {
  return results.map((m) => m.timestamp());
}

// ============================================================================
// E2E Pagination Tests
// ============================================================================

describe('pagination_cursor', () => {
  // Mirrors: test_pagination_cursor_local
  test('LiveQuery pagination: update_selection with cursor', async () => {
    const ctx = setup();
    const roomId = await createRoom(ctx, 'TestRoom');

    // Create 100 messages
    {
      const trx = ctx.begin();
      for (let i = 0; i < 100; i++) {
        const ts = TIMESTAMP_BASE + i * TIMESTAMP_STEP;
        await trx.create(TestMessage, {
          room: roomId.toBase64(),
          text: `Message #${String(i).padStart(3, '0')}`,
          timestamp: ts,
          deleted: false,
        });
      }
      await trx.commit();
    }

    // Initial query: newest 33 messages
    // Rust: let q = format!("room = '{}' AND deleted = false ORDER BY timestamp DESC LIMIT 33", room_id.to_base64());
    const q = `room = '${roomId.toBase64()}' AND deleted = false ORDER BY timestamp DESC LIMIT 33`;
    const lq = await ctx.queryWait(TestMessage, matchArgs(q));

    const items = lq.peek();
    expect(items.length).toBe(33);

    const timestamps = getTimestamps(items);
    const newestTs = Math.max(...timestamps);
    const oldestInPage = Math.min(...timestamps);

    // Rust: assert_eq!(newest_ts, TIMESTAMP_BASE + 99 * TIMESTAMP_STEP);
    expect(newestTs).toBe(TIMESTAMP_BASE + 99 * TIMESTAMP_STEP);
    // Rust: assert_eq!(oldest_in_page, TIMESTAMP_BASE + 67 * TIMESTAMP_STEP);
    expect(oldestInPage).toBe(TIMESTAMP_BASE + 67 * TIMESTAMP_STEP);

    // Pagination: get more messages with timestamp <= newest (expand the window)
    // Rust: lq.update_selection_wait(pagination_q.as_str()).await?;
    const paginationQ = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp <= ${newestTs} ORDER BY timestamp DESC LIMIT 54`;
    await lq.inner.updateSelectionWait(paginationQ);

    const itemsAfter = lq.peek();
    // Rust: assert_eq!(items_after.len(), 54, "After pagination should return 54 (the bug returned only 33)");
    expect(itemsAfter.length).toBe(54);

    lq.drop();
  });

  // Mirrors: test_pagination_forward
  test('LiveQuery pagination: forward pagination with >', async () => {
    const ctx = setup();
    const roomId = await createRoom(ctx, 'TestRoom');

    // Create 100 messages
    {
      const trx = ctx.begin();
      for (let i = 0; i < 100; i++) {
        const ts = TIMESTAMP_BASE + i * TIMESTAMP_STEP;
        await trx.create(TestMessage, {
          room: roomId.toBase64(),
          text: `Message #${String(i).padStart(3, '0')}`,
          timestamp: ts,
          deleted: false,
        });
      }
      await trx.commit();
    }

    // Initial query: oldest 20 messages (ASC order)
    const q = `room = '${roomId.toBase64()}' AND deleted = false ORDER BY timestamp ASC LIMIT 20`;
    const lq = await ctx.queryWait(TestMessage, matchArgs(q));

    const items = lq.peek();
    expect(items.length).toBe(20);

    const timestamps = getTimestamps(items);
    const oldestTs = Math.min(...timestamps);
    const cursorTs = Math.max(...timestamps); // Last item becomes cursor

    // Rust: assert_eq!(oldest_ts, TIMESTAMP_BASE);
    expect(oldestTs).toBe(TIMESTAMP_BASE);
    // Rust: assert_eq!(cursor_ts, TIMESTAMP_BASE + 19 * TIMESTAMP_STEP);
    expect(cursorTs).toBe(TIMESTAMP_BASE + 19 * TIMESTAMP_STEP);

    // Forward pagination: get next page after cursor
    const paginationQ = `room = '${roomId.toBase64()}' AND deleted = false AND timestamp > ${cursorTs} ORDER BY timestamp ASC LIMIT 20`;
    await lq.inner.updateSelectionWait(paginationQ);

    const itemsAfter = lq.peek();
    // Rust: assert_eq!(items_after.len(), 20, "Should get next 20 messages");
    expect(itemsAfter.length).toBe(20);

    const newTimestamps = getTimestamps(itemsAfter);
    // Rust: assert_eq!(new_timestamps[0], TIMESTAMP_BASE + 20 * TIMESTAMP_STEP, "Should start after cursor");
    expect(newTimestamps[0]).toBe(TIMESTAMP_BASE + 20 * TIMESTAMP_STEP);

    lq.drop();
  });

  // Mirrors: test_pagination_inter_node
  // Requires LocalProcessConnection which is not yet ported.
  test.skip('inter-node pagination (requires LocalProcessConnection)', () => {
    // Rust: let server = Node::new_durable(Arc::new(SledStorageEngine::new_test()?), PermissiveAgent::new());
    // Rust: let client = Node::new(Arc::new(SledStorageEngine::new_test()?), PermissiveAgent::new());
    // Rust: let _conn = LocalProcessConnection::new(&server, &client).await?;
    //
    // Creates 100 messages on server, queries from client with nocache,
    // then paginates via update_selection_wait and verifies 54 results.
    // Requires @ankurah/connector-local which is not yet ported.
  });
});

// ============================================================================
// Multi-Column ORDER BY Pagination Tests
// ============================================================================

describe('pagination_cursor - multi-column ORDER BY', () => {
  // Mirrors: test_pagination_multi_column_order_by
  test('multi-column cursor pagination: ORDER BY category ASC, timestamp DESC', async () => {
    const ctx = setup();

    // Create posts with duplicate categories and varying timestamps
    {
      const trx = ctx.begin();
      // Category A: 10 posts
      for (let i = 0; i < 10; i++) {
        const ts = TIMESTAMP_BASE + i * TIMESTAMP_STEP;
        await trx.create(ForumPost, {
          category: 'A',
          timestamp: ts,
          author: `Author ${i % 3}`,
          title: `A Post ${i}`,
        });
      }
      // Category B: 10 posts
      for (let i = 0; i < 10; i++) {
        const ts = TIMESTAMP_BASE + i * TIMESTAMP_STEP;
        await trx.create(ForumPost, {
          category: 'B',
          timestamp: ts,
          author: `Author ${i % 3}`,
          title: `B Post ${i}`,
        });
      }
      // Category C: 10 posts
      for (let i = 0; i < 10; i++) {
        const ts = TIMESTAMP_BASE + i * TIMESTAMP_STEP;
        await trx.create(ForumPost, {
          category: 'C',
          timestamp: ts,
          author: `Author ${i % 3}`,
          title: `C Post ${i}`,
        });
      }
      await trx.commit();
    }

    // First page: ORDER BY category ASC, timestamp DESC LIMIT 15
    // Should get: all 10 A posts (newest first) + 5 B posts (newest first)
    const q = "timestamp > 0 ORDER BY category ASC, timestamp DESC LIMIT 15";
    const lq = await ctx.queryWait(ForumPost, matchArgs(q));

    const items = lq.peek();
    expect(items.length).toBe(15);

    // Verify ordering
    const categories: string[] = items.map((p: any) => p.category());
    const timestamps: number[] = items.map((p: any) => p.timestamp());

    // Rust: assert!(categories[..10].iter().all(|c| c == "A"), "First 10 should be category A");
    expect(categories.slice(0, 10).every((c) => c === 'A')).toBe(true);
    // Rust: assert!(categories[10..15].iter().all(|c| c == "B"), "Next 5 should be category B");
    expect(categories.slice(10, 15).every((c) => c === 'B')).toBe(true);

    // Within each category, timestamps should be DESC
    // Rust: assert!(timestamps[..10].windows(2).all(|w| w[0] >= w[1]), "A posts should be DESC");
    for (let i = 0; i < 9; i++) {
      expect(timestamps[i]).toBeGreaterThanOrEqual(timestamps[i + 1]);
    }
    // Rust: assert!(timestamps[10..15].windows(2).all(|w| w[0] >= w[1]), "B posts should be DESC");
    for (let i = 10; i < 14; i++) {
      expect(timestamps[i]).toBeGreaterThanOrEqual(timestamps[i + 1]);
    }

    // Get the cursor position for next page
    const lastCategory = categories[categories.length - 1];
    const lastTimestamp = timestamps[timestamps.length - 1];
    // Rust: assert_eq!(last_category, "B");
    expect(lastCategory).toBe('B');
    // Rust: assert_eq!(last_timestamp, TIMESTAMP_BASE + 5 * TIMESTAMP_STEP);
    expect(lastTimestamp).toBe(TIMESTAMP_BASE + 5 * TIMESTAMP_STEP);

    // Second page: expand limit to get all items
    // Rust: lq.update_selection_wait(q2).await?;
    const q2 = "timestamp > 0 ORDER BY category ASC, timestamp DESC LIMIT 30";
    await lq.inner.updateSelectionWait(q2);

    const items2 = lq.peek();
    // Rust: assert_eq!(items2.len(), 30, "Should get all 30 posts");
    expect(items2.length).toBe(30);

    const categories2: string[] = items2.map((p: any) => p.category());
    // Rust: assert!(categories2[..10].iter().all(|c| c == "A"), "First 10 still A");
    expect(categories2.slice(0, 10).every((c) => c === 'A')).toBe(true);
    // Rust: assert!(categories2[10..20].iter().all(|c| c == "B"), "Next 10 should be B");
    expect(categories2.slice(10, 20).every((c) => c === 'B')).toBe(true);
    // Rust: assert!(categories2[20..30].iter().all(|c| c == "C"), "Last 10 should be C");
    expect(categories2.slice(20, 30).every((c) => c === 'C')).toBe(true);

    lq.drop();
  });

  // Mirrors: test_pagination_multi_column_with_equality_prefix
  test('multi-column pagination with equality prefix: ORDER BY timestamp DESC, author ASC', async () => {
    const ctx = setup();

    // Create posts with same timestamp but different authors
    {
      const trx = ctx.begin();
      for (let tsOffset = 0; tsOffset < 5; tsOffset++) {
        const ts = TIMESTAMP_BASE + tsOffset * TIMESTAMP_STEP;
        for (let authorIdx = 0; authorIdx < 3; authorIdx++) {
          // Rust: format!("Author_{}", (b'C' - author_idx) as char) -> C, B, A
          const authorChar = String.fromCharCode('C'.charCodeAt(0) - authorIdx);
          await trx.create(ForumPost, {
            category: 'Tech',
            timestamp: ts,
            author: `Author_${authorChar}`,
            title: `Post ts=${tsOffset} author=${authorIdx}`,
          });
        }
      }
      await trx.commit();
    }

    // Query: category = 'Tech' ORDER BY timestamp DESC, author ASC LIMIT 10
    const q = "category = 'Tech' ORDER BY timestamp DESC, author ASC LIMIT 10";
    const lq = await ctx.queryWait(ForumPost, matchArgs(q));

    const items = lq.peek();
    expect(items.length).toBe(10);

    const resultTuples: Array<[number, string]> = items.map((p: any) => [p.timestamp(), p.author()]);

    // Expected order:
    // ts_offset=4: A, B, C (ts DESC first, then author ASC)
    // ts_offset=3: A, B, C
    // ts_offset=2: A (only first from this group to reach 10)

    const ts4 = TIMESTAMP_BASE + 4 * TIMESTAMP_STEP;
    // Rust: assert_eq!(result_tuples[0], (ts_4, "Author_A".to_string()));
    expect(resultTuples[0]).toEqual([ts4, 'Author_A']);
    // Rust: assert_eq!(result_tuples[1], (ts_4, "Author_B".to_string()));
    expect(resultTuples[1]).toEqual([ts4, 'Author_B']);
    // Rust: assert_eq!(result_tuples[2], (ts_4, "Author_C".to_string()));
    expect(resultTuples[2]).toEqual([ts4, 'Author_C']);

    const ts3 = TIMESTAMP_BASE + 3 * TIMESTAMP_STEP;
    // Rust: assert_eq!(result_tuples[3], (ts_3, "Author_A".to_string()));
    expect(resultTuples[3]).toEqual([ts3, 'Author_A']);
    // Rust: assert_eq!(result_tuples[4], (ts_3, "Author_B".to_string()));
    expect(resultTuples[4]).toEqual([ts3, 'Author_B']);
    // Rust: assert_eq!(result_tuples[5], (ts_3, "Author_C".to_string()));
    expect(resultTuples[5]).toEqual([ts3, 'Author_C']);

    lq.drop();
  });
});
