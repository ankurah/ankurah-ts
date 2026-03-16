// MIRRORS: ankurah/tests/tests/json_livequery.rs
//
// Tests for JSON path queries with LiveQuery/subscriptions.
//
// This tests that JSON path predicates (e.g., `context.task_id = ?`) work correctly
// when used with LiveQuery subscriptions, not just fetch().
//
// Issue: JSON path queries work with ctx.fetch() but may fail with ctx.query() because
// the reactor's update_query re-evaluates predicates against Entity::Filterable::value()
// which must properly handle JSON path traversal.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { Node, matchArgs } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText, lww } from '../../src/define-model.ts';
import type { ChangeSet, ChangeKind, ItemChange } from '../../src/changes.ts';
import type { ViewInstance } from '../../src/model.ts';
import { Json } from '../../src/property/value/json.ts';

// ── Model ──
// Mirrors: json_livequery.rs `struct Message { content: String, context: Json }`
// Divergence: Json property is stored as LWW<Json> since Json uses LWW semantics in Rust [E1].
const Message = defineModel('message', {
  content: yrsText(),
  context: lww<unknown>(),
});

// ── TestWatcher ──
// Mirrors: common.rs TestWatcher — accumulates changeset notifications with async waiting.

class TestWatcher {
  private batches: Array<Array<[string, ChangeKind]>> = [];
  private resolvers: Array<() => void> = [];

  listener(): (changeset: ChangeSet<ViewInstance>) => void {
    return (changeset: ChangeSet<ViewInstance>) => {
      const batch: Array<[string, ChangeKind]> = changeset.changes.map(
        (change: ItemChange<ViewInstance>) => [change.item.id().toBase64(), change.kind] as [string, ChangeKind],
      );
      this.batches.push(batch);
      for (const resolve of this.resolvers) resolve();
      this.resolvers = [];
    };
  }

  async wait(timeoutMs = 10000): Promise<boolean> {
    if (this.batches.length > 0) return true;
    return new Promise<boolean>((resolve) => {
      const timer = setTimeout(() => resolve(false), timeoutMs);
      this.resolvers.push(() => {
        clearTimeout(timer);
        resolve(true);
      });
    });
  }

  async quiesce(): Promise<number> {
    await new Promise((resolve) => setTimeout(resolve, 100));
    return this.batches.length;
  }
}

// ── Helper ──

function createTestNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
}

// ── Tests ──

describe('json_livequery', () => {
  // Mirrors: json_livequery.rs test_json_path_livequery_initial_results
  test('test_json_path_livequery_initial_results', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Create messages with different task_id contexts
    const taskIdA = 'task-aaa';
    const taskIdB = 'task-bbb';

    {
      const trx = ctx.begin();
      await trx.create(Message, { content: 'Message 1 for task A', context: { task_id: taskIdA } });
      await trx.create(Message, { content: 'Message 2 for task A', context: { task_id: taskIdA } });
      await trx.create(Message, { content: 'Message for task B', context: { task_id: taskIdB } });
      await trx.commit();
    }

    // Verify fetch works (baseline)
    const queryStr = `context.task_id = '${taskIdA}'`;
    const fetched = await ctx.fetch(Message, matchArgs(queryStr));
    expect(fetched.length).toBe(2);

    // Now test LiveQuery - this is where the bug manifests
    const query = await ctx.queryWait(Message, matchArgs(queryStr));

    const items = query.peek();
    expect(items.length).toBe(2);
  });

  // Mirrors: json_livequery.rs test_json_path_livequery_with_new_entity
  test('test_json_path_livequery_with_new_entity', async () => {
    const node = createTestNode();
    const ctx = node.context();

    const taskId = 'task-xyz';

    // Set up LiveQuery before creating any entities
    const queryStr = `context.task_id = '${taskId}'`;
    const query = await ctx.queryWait(Message, matchArgs(queryStr));

    const watcher = new TestWatcher();
    const _handle = query.subscribe(watcher.listener());

    // Initially empty
    expect(query.peek().length).toBe(0);

    // Create a matching message
    {
      const trx = ctx.begin();
      await trx.create(Message, { content: 'New message', context: { task_id: taskId } });
      await trx.commit();
    }

    // Wait for notification and check
    expect(await watcher.wait()).toBe(true);

    const items = query.peek();
    expect(items.length).toBe(1);
    expect(items[0].content()).toBe('New message');
  });

  // Mirrors: json_livequery.rs test_json_path_livequery_with_nested_path
  test('test_json_path_livequery_with_nested_path', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Create message with nested JSON context
    {
      const trx = ctx.begin();
      await trx.create(Message, {
        content: 'Nested context message',
        context: {
          refs: {
            task_id: 'nested-task',
            user_id: 'user-123',
          },
        },
      });
      await trx.commit();
    }

    // Query with nested path
    const query = await ctx.queryWait(Message, matchArgs("context.refs.task_id = 'nested-task'"));

    const items = query.peek();
    expect(items.length).toBe(1);
    expect(items[0].content()).toBe('Nested context message');
  });

  // Mirrors: json_livequery.rs test_json_path_predicate_reevaluation
  test('test_json_path_predicate_reevaluation', async () => {
    const node = createTestNode();
    const ctx = node.context();

    const taskId = 'reevaluation-test';

    // Create entity first
    {
      const trx = ctx.begin();
      await trx.create(Message, {
        content: 'Test message',
        context: { task_id: taskId, extra: 'data' },
      });
      await trx.commit();
    }

    // Now create LiveQuery - this tests the path where:
    // 1. Storage returns the entity (it matches the indexed query)
    // 2. update_query re-evaluates the predicate using Entity::Filterable::value()
    // 3. The predicate must correctly extract context.task_id from the Entity
    const queryStr = `context.task_id = '${taskId}'`;
    const query = await ctx.queryWait(Message, matchArgs(queryStr));

    const items = query.peek();

    // This is the key assertion - if this fails, the predicate re-evaluation is broken
    expect(items.length).toBe(1);
  });
});
