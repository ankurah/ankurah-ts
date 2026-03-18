// MIRRORS: ankurah/storage/postgres/tests/basic.rs

import { describe, test, expect, beforeAll, afterAll } from 'bun:test';
import { matchArgs } from '@ankurah/core';
import {
  createPostgresContainer,
  stopPostgresContainer,
  createPostgresNode,
  Album,
  type PostgresTestContext,
} from './common.ts';

let pgCtx: PostgresTestContext;

beforeAll(async () => {
  pgCtx = await createPostgresContainer();
}, 60_000);

afterAll(async () => {
  await stopPostgresContainer(pgCtx);
}, 30_000);

describe('basic postgres', () => {
  // Rust: fn test_postgres
  test('test_postgres', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    const trx = ctx.begin();
    await trx.create(Album, { name: 'The rest of the owl', year: '2024' });
    await trx.commit();
  });
});
