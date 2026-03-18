// MIRRORS: ankurah/storage/sqlite/tests/basic.rs
//
// SQLite Storage Integration Tests
//
// These tests verify that the SQLite storage engine works correctly with entity mutations,
// including:
// - Creating entities
// - Updating entities
// - Querying entities
// - State change detection

import { describe, test, expect } from 'bun:test';
import { matchArgs } from '@ankurah/core';
import { createSqliteNode, Album, TestWatcher } from './common.ts';

describe('basic SQLite integration', () => {
  test('test_sqlite_create_and_query', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Create some albums
    {
      const trx = ctx.begin();
      await trx.create(Album, { name: 'Album 1', year: '2020' });
      await trx.create(Album, { name: 'Album 2', year: '2021' });
      await trx.create(Album, { name: 'Album 3', year: '2022' });
      await trx.commit();
    }

    // Query albums
    const albums = await ctx.fetch(Album, matchArgs("year > '2020'"));
    expect(albums.length).toBe(2);
    expect(albums.some((a) => a.name() === 'Album 2')).toBe(true);
    expect(albums.some((a) => a.name() === 'Album 3')).toBe(true);
  });

  test('test_sqlite_update_entity', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Create an album
    let albumId;
    {
      const trx = ctx.begin();
      const albumBorrow = await trx.create(Album, { name: 'Original Name', year: '2020' });
      albumId = albumBorrow.inner.id();
      await trx.commit();
    }

    // Update the album
    {
      const trx = ctx.begin();
      const albumMut = await trx.get(Album, albumId);
      // Divergence: Rust uses album.edit(&trx).unwrap().name().overwrite(0, 13, "Updated Name")
      // TS uses Yjs backend directly [E5]
      const yjs = albumMut.inner.entity().getBackend((await import('@ankurah/core')).YjsBackend);
      yjs.delete('name', 0, 13);
      yjs.insert('name', 0, 'Updated Name');
      await trx.commit();
    }

    // Verify the update
    const albums = await ctx.fetch(Album, matchArgs("name = 'Updated Name'"));
    expect(albums.length).toBe(1);
    expect(albums[0].name()).toBe('Updated Name');
    expect(albums[0].year()).toBe('2020');
  });

  test('test_sqlite_state_change_detection', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Create an album
    let albumId;
    {
      const trx = ctx.begin();
      const albumBorrow = await trx.create(Album, { name: 'Test Album', year: '2020' });
      albumId = albumBorrow.inner.id();
      await trx.commit();
    }

    // First update should change state
    {
      const trx = ctx.begin();
      const albumMut = await trx.get(Album, albumId);
      const yjs = albumMut.inner.entity().getBackend((await import('@ankurah/core')).YjsBackend);
      yjs.delete('name', 0, 10);
      yjs.insert('name', 0, 'Updated');
      await trx.commit();
    }

    // Verify the update was applied
    const albums = await ctx.fetch(Album, matchArgs("name = 'Updated'"));
    expect(albums.length).toBe(1);
  });

  test('test_sqlite_multiple_updates', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Create multiple albums
    let album1Id, album2Id;
    {
      const trx = ctx.begin();
      const a1 = await trx.create(Album, { name: 'Album 1', year: '2020' });
      const a2 = await trx.create(Album, { name: 'Album 2', year: '2021' });
      album1Id = a1.inner.id();
      album2Id = a2.inner.id();
      await trx.commit();
    }

    // Update both albums
    {
      const trx = ctx.begin();
      const { YjsBackend } = await import('@ankurah/core');
      const a1Mut = await trx.get(Album, album1Id);
      const a2Mut = await trx.get(Album, album2Id);
      const yjs1 = a1Mut.inner.entity().getBackend(YjsBackend);
      const yjs2 = a2Mut.inner.entity().getBackend(YjsBackend);
      yjs1.delete('name', 0, 7);
      yjs1.insert('name', 0, 'Updated 1');
      yjs2.delete('name', 0, 7);
      yjs2.insert('name', 0, 'Updated 2');
      await trx.commit();
    }

    // Verify both updates
    const albums1 = await ctx.fetch(Album, matchArgs("name = 'Updated 1'"));
    const albums2 = await ctx.fetch(Album, matchArgs("name = 'Updated 2'"));
    expect(albums1.length).toBe(1);
    expect(albums2.length).toBe(1);
  });

  test('test_sqlite_query_with_subscription', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    const watcher = new TestWatcher();
    const query = await ctx.queryWait(Album, matchArgs("year > '2020'"));
    const _handle = query.subscribe(watcher.listener());

    // Create albums before subscription (these trigger subscription since query was already set up)
    {
      const trx = ctx.begin();
      await trx.create(Album, { name: 'Album 1', year: '2021' });
      await trx.create(Album, { name: 'Album 2', year: '2022' });
      await trx.commit();
    }

    // Wait a bit for notifications
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Create another album that matches (should trigger subscription)
    {
      const trx = ctx.begin();
      await trx.create(Album, { name: 'Album 3', year: '2023' });
      await trx.commit();
    }

    // Should receive notification for the new album
    const changes = await watcher.takeOne();
    expect(changes.length).toBeGreaterThanOrEqual(1);
  });
});
