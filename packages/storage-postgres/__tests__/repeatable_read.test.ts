// MIRRORS: ankurah/storage/postgres/tests/repeatable_read.rs

import { describe, test, expect, beforeAll, beforeEach, afterAll } from 'bun:test';
import { YjsBackend } from '@ankurah/core';
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

describe('repeatable_read', () => {
  // Each Rust test creates a fresh container.
  beforeEach(async () => {
    await pgCtx.engine.deleteAllCollections();
  });

  // Rust: fn pg_repeatable_read
  test('pg_repeatable_read', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Create Album "I love cats"
    let albumId;
    {
      const trx = ctx.begin();
      const albumBorrow = await trx.create(Album, { name: 'I love cats', year: '2024' });
      albumId = albumBorrow.inner.id();
      // Verify initial value
      const yjs = albumBorrow.inner.entity().getBackend(YjsBackend);
      expect(yjs.getString('name')).toBe('I love cats');
      await trx.commit();
    }

    // Open read-only view
    const albumRo = await ctx.get(Album, albumId);
    expect(albumRo.name()).toBe('I love cats');

    // Start two concurrent transactions
    const trx2 = ctx.begin();
    const album2 = await trx2.get(Album, albumId);

    const trx3 = ctx.begin();
    const album3 = await trx3.get(Album, albumId);

    // tx2: cats -> tofu
    const yjs2 = album2.inner.entity().getBackend(YjsBackend);
    yjs2.delete('name', 7, 4);
    yjs2.insert('name', 7, 'tofu');
    expect(yjs2.getString('name')).toBe('I love tofu');

    // tx3: love -> devour
    const yjs3 = album3.inner.entity().getBackend(YjsBackend);
    yjs3.delete('name', 2, 4);
    yjs3.insert('name', 2, 'devour');
    expect(yjs3.getString('name')).toBe('I devour cats');

    // Uncommitted changes should not affect read view
    expect(albumRo.name()).toBe('I love cats');

    await trx2.commit();
    // After trx2 commit, view should update
    expect(albumRo.name()).toBe('I love tofu');

    await trx3.commit();
    // After trx3 commit, CRDT merge: "I devour tofu"
    expect(albumRo.name()).toBe('I devour tofu');
  });

  // Rust: fn pg_events
  test('pg_events', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    let albumId;
    {
      const trx = ctx.begin();
      const albumBorrow = await trx.create(Album, { name: 'I love cats', year: '2024' });
      albumId = albumBorrow.inner.id();
      const yjs = albumBorrow.inner.entity().getBackend(YjsBackend);
      expect(yjs.getString('name')).toBe('I love cats');
      await trx.commit();
    }

    const albumRo = await ctx.get(Album, albumId);

    const trx2 = ctx.begin();
    const album2 = await trx2.get(Album, albumId);

    const trx3 = ctx.begin();
    const album3 = await trx3.get(Album, albumId);

    // tx2: cats -> tofu
    const yjs2 = album2.inner.entity().getBackend(YjsBackend);
    yjs2.delete('name', 7, 4);
    yjs2.insert('name', 7, 'tofu');
    expect(yjs2.getString('name')).toBe('I love tofu');

    // tx3: love -> devour
    const yjs3 = album3.inner.entity().getBackend(YjsBackend);
    yjs3.delete('name', 2, 4);
    yjs3.insert('name', 2, 'devour');
    expect(yjs3.getString('name')).toBe('I devour cats');

    // Uncommitted changes should not affect read view
    expect(albumRo.name()).toBe('I love cats');

    await trx2.commit();
    expect(albumRo.name()).toBe('I love tofu');

    await trx3.commit();
    expect(albumRo.name()).toBe('I devour tofu');
  });
});
