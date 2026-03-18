// MIRRORS: ankurah/storage/postgres/tests/add_event.rs

import { describe, test, expect, beforeAll, afterAll } from 'bun:test';
import { YjsBackend } from '@ankurah/core';
import { CollectionId } from '@ankurah/proto';
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

describe('add_event postgres', () => {
  // Rust: fn add_event_postgres
  test('add_event_postgres', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Create an Album, commit
    const trx = ctx.begin();
    const albumBorrow = await trx.create(Album, { name: 'The rest of the owl', year: '2024' });
    const albumId = albumBorrow.inner.id();
    await trx.commit();

    // Edit the album (insert text), commit
    const trx1 = ctx.begin();
    const album1 = await trx1.get(Album, albumId);
    const yjs1 = album1.inner.entity().getBackend(YjsBackend);
    yjs1.insert('name', 0, '(o.');
    await trx1.commit();

    // Edit the album again (insert text), commit
    const trx2 = ctx.begin();
    const album2 = await trx2.get(Album, albumId);
    const yjs2 = album2.inner.entity().getBackend(YjsBackend);
    yjs2.insert('name', 3, 'o) ');
    await trx2.commit();

    // Dump entity events — should have 3 events
    const collection = await ctx.collection(CollectionId.from('album'));
    const events = await collection.dumpEntityEvents(albumId);
    expect(events.length).toBe(3);
  });
});
