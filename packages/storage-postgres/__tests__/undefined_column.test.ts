// MIRRORS: ankurah/storage/postgres/tests/undefined_column.rs
//
// Test for undefined column handling in postgres queries.
// Queries referencing columns that don't exist yet are handled gracefully
// by treating missing columns as NULL via schema-based filtering.

import { describe, test, expect, beforeAll, beforeEach, afterAll } from 'bun:test';
import { matchArgs } from '@ankurah/core';
import {
  createPostgresContainer,
  stopPostgresContainer,
  createPostgresNode,
  Postgres,
  Task,
  type PostgresTestContext,
} from './common.ts';

let pgCtx: PostgresTestContext;

beforeAll(async () => {
  pgCtx = await createPostgresContainer();
}, 60_000);

afterAll(async () => {
  await stopPostgresContainer(pgCtx);
}, 30_000);

// Each Rust test creates a fresh container. Since we share one container,
// clean all tables before each test so system.create() works fresh.
beforeEach(async () => {
  await pgCtx.engine.deleteAllCollections();
});

describe('undefined_column', () => {
  // Rust: fn test_undefined_column_in_where
  test('test_undefined_column_in_where', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Query for tasks with status filter — but no tasks exist, column doesn't exist
    // Should return empty results, not error
    const results = await ctx.fetch(Task, matchArgs("status = 'active'"));
    expect(results.length).toBe(0);
  });

  // Rust: fn test_undefined_column_in_order_by
  test('test_undefined_column_in_order_by', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Query with ORDER BY on a column that doesn't exist yet
    const results = await ctx.fetch(Task, matchArgs("name = 'nonexistent' ORDER BY created DESC"));
    expect(results.length).toBe(0);
  });

  // Rust: fn test_undefined_columns_where_and_order_by
  test('test_undefined_columns_where_and_order_by', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Both "status" (WHERE) and "created" (ORDER BY) don't exist
    const results = await ctx.fetch(Task, matchArgs("status = 'pending' OR status = 'active' ORDER BY created DESC"));
    expect(results.length).toBe(0);
  });

  // Rust: fn test_columns_exist_after_write
  test('test_columns_exist_after_write', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Create a Task (creates columns)
    {
      const trx = ctx.begin();
      await trx.create(Task, { name: 'Test task', status: 'pending', created: '2024-01-01' });
      await trx.commit();
    }

    // Query with status filter and ORDER BY created — should return the created task
    const results = await ctx.fetch(Task, matchArgs("status = 'pending' ORDER BY created DESC"));
    expect(results.length).toBe(1);
    expect(results[0].name()).toBe('Test task');
  });

  // Rust: fn test_cache_refresh_after_column_creation
  test('test_cache_refresh_after_column_creation', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Query before data — empty results
    const results = await ctx.fetch(Task, matchArgs("status = 'pending'"));
    expect(results.length).toBe(0);

    // Write creates columns
    {
      const trx = ctx.begin();
      await trx.create(Task, { name: 'Task 1', status: 'pending', created: '2024-01-01' });
      await trx.commit();
    }

    // Query again — cache refreshes and finds column, returns result
    const results2 = await ctx.fetch(Task, matchArgs("status = 'pending'"));
    expect(results2.length).toBe(1);
  });
});
