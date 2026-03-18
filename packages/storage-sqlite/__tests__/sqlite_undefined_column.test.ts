// MIRRORS: ankurah/storage/sqlite/tests/sqlite_undefined_column.rs
//
// Test for undefined column handling in SQLite queries
//
// Tests that queries referencing columns that don't exist yet are handled gracefully
// by treating missing columns as NULL via schema-based filtering.
//
// These tests mirror pg_undefined_column.rs to ensure SQLite has parity with Postgres.

import { describe, test, expect } from 'bun:test';
import { matchArgs } from '@ankurah/core';
import { createSqliteNode, Task } from './common.ts';

describe('sqlite_undefined_column', () => {
  test('test_undefined_column_in_where', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Query for tasks with status filter - but no tasks exist yet, so the column doesn't exist
    // This should return empty results, not error
    const results = await ctx.fetch(Task, matchArgs("status = 'active'"));
    expect(results.length).toBe(0);
  });

  test('test_undefined_column_in_order_by', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Query with ORDER BY on a column that doesn't exist yet
    const results = await ctx.fetch(Task, matchArgs("name = 'nonexistent' ORDER BY created DESC"));
    expect(results.length).toBe(0);
  });

  test('test_undefined_columns_where_and_order_by', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Both "status" (WHERE) and "created" (ORDER BY) don't exist
    // Schema-based filtering treats both as NULL upfront (no retry needed)
    const results = await ctx.fetch(Task, matchArgs("status = 'pending' OR status = 'active' ORDER BY created DESC"));
    expect(results.length).toBe(0);
  });

  test('test_columns_exist_after_write', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // First, create a task - this should create the columns
    {
      const trx = ctx.begin();
      await trx.create(Task, { name: 'Test task', status: 'pending', created: '2024-01-01' });
      await trx.commit();
    }

    // Now the query should work
    const results = await ctx.fetch(Task, matchArgs("status = 'pending' ORDER BY created DESC"));
    expect(results.length).toBe(1);
    expect(results[0].name()).toBe('Test task');
  });

  test('test_cache_refresh_after_column_creation', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Query before any data - columns don't exist, should get empty results
    const results = await ctx.fetch(Task, matchArgs("status = 'pending'"));
    expect(results.length).toBe(0);

    // Write creates the columns
    {
      const trx = ctx.begin();
      await trx.create(Task, { name: 'Task 1', status: 'pending', created: '2024-01-01' });
      await trx.commit();
    }

    // Query again - cache should refresh and find the column now exists
    const results2 = await ctx.fetch(Task, matchArgs("status = 'pending'"));
    expect(results2.length).toBe(1);
  });
});
