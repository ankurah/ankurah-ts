// MIRRORS: ankurah/tests/tests/concurrent_transactions.rs
//
// Tests for concurrent transactions modifying the same entity.
// Verifies that multiple transactions forking from the same head
// commit correctly and produce expected merged state.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { Node, matchArgs } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';
import { YrsString } from '../../src/property/value/yrs_string.ts';

// ── Model ──
// Mirrors: common.rs Album { name: String, year: String }
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// ── Helpers ──

function createDurableNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
}

function getYrsStringHandle(entity: import('../../src/entity.ts').Entity, fieldName: string): YrsString {
  const backend = entity.getBackend(YjsBackend);
  return new YrsString(fieldName, backend, entity);
}

// ── Tests ──

describe('concurrent_transactions', () => {
  // Mirrors: concurrent_transactions.rs test_concurrent_transactions_same_entity
  test('test_concurrent_transactions_same_entity', async () => {
    const node = createDurableNode();
    const context = node.context();

    // Create initial entity
    let albumId;
    {
      const trx = context.begin();
      const albumBorrow = await trx.create(Album, { name: 'Initial Name', year: '2024' });
      albumId = albumBorrow.inner.id();
      await trx.commit();
    }

    // Get the entity so both transactions will fork from the same head
    const album = await context.get(Album, albumId);

    // Start two concurrent transactions
    const trx1 = context.begin();
    const trx2 = context.begin();

    // Both transactions edit the same entity (both fork from same head)
    // Rust: album.edit(&trx) — TS: trx.edit(Model, entity)
    const albumMut1 = trx1.edit(Album, album.entity());
    const albumMut2 = trx2.edit(Album, album.entity());

    // Make different changes
    getYrsStringHandle(albumMut1.inner.entity(), 'name').replace('Updated by Trx1');
    getYrsStringHandle(albumMut2.inner.entity(), 'year').replace('2025');

    // Commit first transaction - this should succeed
    await trx1.commit();

    // Commit second transaction - this should handle the concurrent update
    // The second transaction's event has parent that equals the head before trx1 committed,
    // but now the head has been updated by trx1. This should be detected as NotDescends
    // and handled appropriately.
    await trx2.commit();

    // Verify both changes were applied via the live entity view
    const finalAlbum = await context.get(Album, albumId);
    expect(finalAlbum.name()).toBe('Updated by Trx1');
    expect(finalAlbum.year()).toBe('2025');

    // Persisted state must include both concurrent commits in its head
    const collection = await context.collection(Album.collection());
    const storedState = await collection.getState(albumId);
    const persistedHead = storedState.payload.state.head;
    expect(persistedHead.len()).toBe(2);

    // All head entries must correspond to stored events
    const persistedEvents = await collection.dumpEntityEvents(albumId);
    const persistedEventIds = persistedEvents.map((e) => e.payload.id());
    for (const headId of persistedHead.iter()) {
      expect(persistedEventIds.some((eventId) => eventId.equals(headId))).toBe(true);
    }
  });

  // Mirrors: concurrent_transactions.rs test_many_concurrent_transactions
  test('test_many_concurrent_transactions', async () => {
    const node = createDurableNode();
    const context = node.context();

    // Create initial entity
    let albumId;
    {
      const trx = context.begin();
      const albumBorrow = await trx.create(Album, { name: 'Counter', year: '0' });
      albumId = albumBorrow.inner.id();
      await trx.commit();
    }

    const album = await context.get(Album, albumId);

    // Create 5 concurrent transactions
    // Divergence: Rust uses tokio::spawn for parallelism; JS is single-threaded,
    // so we create all transactions and commit them sequentially [E8].
    const promises: Promise<void>[] = [];
    let successes = 0;
    let failures = 0;

    for (let i = 0; i < 5; i++) {
      const trx = context.begin();
      const albumMut = trx.edit(Album, album.entity());
      getYrsStringHandle(albumMut.inner.entity(), 'year').replace(`${i}`);
      promises.push(
        trx.commit().then(
          () => { successes++; },
          (e: unknown) => {
            failures++;
            const errorStr = String(e);
            if (errorStr.includes('BudgetExceeded')) {
              throw new Error(`Got BudgetExceeded error in concurrent transactions: ${errorStr}`);
            }
          },
        ),
      );
    }

    await Promise.all(promises);

    // At least the first transaction should succeed
    expect(successes).toBeGreaterThanOrEqual(1);
  });

  // Mirrors: concurrent_transactions.rs test_concurrent_transactions_long_lineage
  test('test_concurrent_transactions_long_lineage', async () => {
    const node = createDurableNode();
    const context = node.context();

    // Create initial entity and build up a long lineage
    let albumId;
    {
      const trx = context.begin();
      const albumBorrow = await trx.create(Album, { name: 'Initial', year: '0' });
      albumId = albumBorrow.inner.id();
      await trx.commit();
    }

    // Make 20 sequential updates to build lineage
    for (let i = 1; i <= 20; i++) {
      const album = await context.get(Album, albumId);
      const trx = context.begin();
      const albumMut = trx.edit(Album, album.entity());
      getYrsStringHandle(albumMut.inner.entity(), 'year').replace(`${i}`);
      await trx.commit();
    }

    // Now create concurrent transactions that both fork from the same (latest) head
    const album = await context.get(Album, albumId);

    const trx1 = context.begin();
    const trx2 = context.begin();

    const albumMut1 = trx1.edit(Album, album.entity());
    const albumMut2 = trx2.edit(Album, album.entity());

    getYrsStringHandle(albumMut1.inner.entity(), 'name').replace('Updated by Trx1');
    getYrsStringHandle(albumMut2.inner.entity(), 'name').replace('Updated by Trx2');

    // Commit first transaction
    await trx1.commit();

    // Commit second transaction - this should handle concurrency correctly
    // With the bug, this will try to traverse all the way back to root and hit BudgetExceeded
    try {
      await trx2.commit();
    } catch (e: unknown) {
      const errorStr = String(e);
      // This is the bug we're looking for
      if (errorStr.includes('BudgetExceeded')) {
        throw new Error(`Hit BudgetExceeded due to traversing too far back! Error: ${errorStr}`);
      }
      // Other errors are acceptable for concurrent commits
    }
  });
});
