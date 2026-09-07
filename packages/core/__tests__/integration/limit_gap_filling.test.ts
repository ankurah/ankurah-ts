// MIRRORS: ankurah/tests/tests/limit_gap_filling.rs
//
// Tests for ORDER BY + LIMIT gap filling on a single durable node.
//
// Verifies that when entities are removed from a limited result set,
// the gaps are automatically filled by fetching additional entities from local storage.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { LocalProcessConnection } from '@ankurah/connector-local';
import type { EntityId } from '@ankurah/proto';
import { Selection_tryFrom } from '@ankurah/ankql';
import { Node, matchArgs, nocache } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import type { ChangeSet, ChangeKind, ItemChange } from '../../src/changes.ts';
import type { ViewInstance } from '../../src/model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';
import { YrsString } from '../../src/property/value/yrs_string.ts';

// ── Model ──
// Mirrors: common.rs Album { name: String, year: String }
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// ── TestWatcher ──
// Mirrors: common.rs TestWatcher::changeset() — transforms ChangeSet to Vec<(EntityId, ChangeKind)>

class ChangesetWatcher {
  private batches: Array<Array<[EntityId, ChangeKind]>> = [];
  private resolvers: Array<() => void> = [];

  listener(): (changeset: ChangeSet<ViewInstance>) => void {
    return (changeset: ChangeSet<ViewInstance>) => {
      const batch: Array<[EntityId, ChangeKind]> = changeset.changes.map(
        (change: ItemChange<ViewInstance>) => [change.item.id(), change.kind] as [EntityId, ChangeKind],
      );
      this.batches.push(batch);
      for (const resolve of this.resolvers) resolve();
      this.resolvers = [];
    };
  }

  async takeOne(timeoutMs = 10000): Promise<Array<[EntityId, ChangeKind]>> {
    if (this.batches.length > 0) {
      return this.batches.shift()!;
    }
    return new Promise<Array<[EntityId, ChangeKind]>>((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error(`takeOne() timed out after ${timeoutMs}ms (got ${this.batches.length} batches)`));
      }, timeoutMs);
      this.resolvers.push(() => {
        clearTimeout(timeout);
        resolve(this.batches.shift()!);
      });
    });
  }

  async quiesce(): Promise<number> {
    await new Promise((resolve) => setTimeout(resolve, 100));
    return this.batches.length;
  }
}

// ── Helpers ──

function createDurableNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
}

// Mirrors: common.rs create_albums()
async function createAlbums(
  ctx: ReturnType<Node['context']>,
  startYear: number,
  endYear: number,
): Promise<EntityId[]> {
  const trx = ctx.begin();
  const ids: EntityId[] = [];
  for (let year = startYear; year <= endYear; year++) {
    const borrow = await trx.create(Album, { name: `Album ${year}`, year: String(year) });
    ids.push(borrow.inner.id());
  }
  await trx.commit();
  return ids;
}

// Mirrors: common.rs years()
function years(query: { peek(): ViewInstance[] }): string[] {
  return query.peek().map((a: any) => a.year() ?? '');
}

// Helper to get a YrsString handle for mutation
function getYrsStringHandle(entity: import('../../src/entity.ts').Entity, fieldName: string): YrsString {
  const backend = entity.getBackend(YjsBackend);
  return new YrsString(fieldName, backend, entity);
}

// ── Tests ──

describe('limit_gap_filling', () => {
  // Mirrors: limit_gap_filling.rs test_single_node_gap_filling
  test('test_single_node_gap_filling', async () => {
    const node = createDurableNode();
    const ctx = node.context();
    const ids = await createAlbums(ctx, 2020, 2024);

    const watcher = new ChangesetWatcher();
    const query = await ctx.queryWait(Album, matchArgs("year >= '2020' ORDER BY year ASC LIMIT 3"));
    const _handle = query.subscribe(watcher.listener());

    // Initial state should have the first 3 albums (2020, 2021, 2022)
    expect(await watcher.quiesce()).toBe(0);
    expect(query.peek().length).toBe(3);
    expect(years(query)).toEqual(['2020', '2021', '2022']);

    // Update the middle album (2021) to no longer match - this should trigger gap filling
    {
      const trx = ctx.begin();
      const albumBorrow = await trx.get(Album, ids[1]);
      // Rust: trx.get::<Album>(&ids[1]).await?.year().replace("1999")?;
      getYrsStringHandle(albumBorrow.inner.entity(), 'year').replace('1999');
      await trx.commit();
    }

    // Wait for gap filling: remove notification for updated album, then add notification for 2023
    const changes = await watcher.takeOne();
    expect(changes.length).toBe(2);
    expect(changes[0][0].equals(ids[1])).toBe(true);
    expect(changes[0][1]).toBe('Remove');
    expect(changes[1][0].equals(ids[3])).toBe(true);
    expect(changes[1][1]).toBe('Add');

    // Final state should have 2020, 2022, 2023 (gap filled with 2023)
    expect(years(query)).toEqual(['2020', '2022', '2023']);
    expect(await watcher.quiesce()).toBe(0);
  });

  // Mirrors: limit_gap_filling.rs test_single_node_multiple_gap_filling
  test('test_single_node_multiple_gap_filling', async () => {
    const node = createDurableNode();
    const ctx = node.context();
    const ids = await createAlbums(ctx, 2020, 2030);

    const watcher = new ChangesetWatcher();
    const query = await ctx.queryWait(Album, matchArgs("year >= '2020' ORDER BY year ASC LIMIT 5"));
    const _handle = query.subscribe(watcher.listener());

    expect(await watcher.quiesce()).toBe(0);
    expect(years(query)).toEqual(['2020', '2021', '2022', '2023', '2024']);

    // Update two albums (2021 and 2023) to no longer match - this should trigger gap filling for both
    const trx = ctx.begin();
    const album1Borrow = await trx.get(Album, ids[1]);
    getYrsStringHandle(album1Borrow.inner.entity(), 'year').replace('1999');
    const album3Borrow = await trx.get(Album, ids[3]);
    getYrsStringHandle(album3Borrow.inner.entity(), 'year').replace('1999');
    await trx.commit();

    // Wait for consolidated gap filling update: 2 removes + 2 adds in one update
    const changes = await watcher.takeOne();
    expect(changes.length).toBe(4);
    expect(changes[0][0].equals(ids[1])).toBe(true);
    expect(changes[0][1]).toBe('Remove'); // 2021
    expect(changes[1][0].equals(ids[3])).toBe(true);
    expect(changes[1][1]).toBe('Remove'); // 2023
    expect(changes[2][0].equals(ids[5])).toBe(true);
    expect(changes[2][1]).toBe('Add');    // 2025
    expect(changes[3][0].equals(ids[6])).toBe(true);
    expect(changes[3][1]).toBe('Add');    // 2026

    expect(years(query)).toEqual(['2020', '2022', '2024', '2025', '2026']);
    expect(await watcher.quiesce()).toBe(0);
  });

  // TS-ONLY: No Rust equivalent — test_inter_node_gap_filling does not exist in limit_gap_filling.rs.
  // Requires SubscriptionRelay to propagate server-side changes to client's LiveQuery.
  // SubscriptionRelay is not yet ported (Phase 1: hasRelay always false).
  test.skip('test_inter_node_gap_filling (requires SubscriptionRelay)', async () => {
    const server = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: true,
    });
    await server.system.create();
    const client = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: false,
    });
    const conn = await LocalProcessConnection.new(server, client);
    await client.system.waitSystemReady();

    const serverCtx = await server.contextAsync();
    const clientCtx = await client.contextAsync();
    const ids = await createAlbums(serverCtx, 2020, 2025);

    const watcher = new ChangesetWatcher();
    const query = await clientCtx.queryWait(Album, nocache("year >= '2020' ORDER BY year ASC LIMIT 3", Selection_tryFrom));
    const _handle = query.subscribe(watcher.listener());

    // Initial state should have the first 3 albums (2020, 2021, 2022)
    expect(await watcher.quiesce()).toBe(0);
    expect(years(query)).toEqual(['2020', '2021', '2022']);

    {
      const trx = serverCtx.begin();
      const albumBorrow = await trx.get(Album, ids[1]);
      getYrsStringHandle(albumBorrow.inner.entity(), 'year').replace('1999');
      await trx.commit();
    }

    const changes = await watcher.takeOne();
    expect(changes.length).toBe(2);
    expect(changes[0][0].equals(ids[1])).toBe(true);
    expect(changes[0][1]).toBe('Remove');
    expect(changes[1][0].equals(ids[3])).toBe(true);
    expect(changes[1][1]).toBe('Add');

    expect(years(query)).toEqual(['2020', '2022', '2023']);
    expect(await watcher.quiesce()).toBe(0);

    conn.destroy();
  });

  // TS-ONLY: No Rust equivalent — test_inter_node_gap_filling_desc does not exist in limit_gap_filling.rs.
  // Requires SubscriptionRelay to propagate server-side changes to client's LiveQuery.
  // SubscriptionRelay is not yet ported (Phase 1: hasRelay always false).
  test.skip('test_inter_node_gap_filling_desc (requires SubscriptionRelay)', async () => {
    const server = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: true,
    });
    await server.system.create();
    const client = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: false,
    });
    const conn = await LocalProcessConnection.new(server, client);
    await client.system.waitSystemReady();

    const serverCtx = await server.contextAsync();
    const clientCtx = await client.contextAsync();
    const ids = await createAlbums(serverCtx, 2020, 2027);

    const watcher = new ChangesetWatcher();
    const query = await clientCtx.queryWait(Album, nocache("year >= '2020' ORDER BY year DESC LIMIT 4", Selection_tryFrom));
    const _handle = query.subscribe(watcher.listener());

    expect(await watcher.quiesce()).toBe(0);
    expect(years(query)).toEqual(['2027', '2026', '2025', '2024']);

    {
      const trx = serverCtx.begin();
      const album4Borrow = await trx.get(Album, ids[4]);
      getYrsStringHandle(album4Borrow.inner.entity(), 'year').replace('1999');
      const album6Borrow = await trx.get(Album, ids[6]);
      getYrsStringHandle(album6Borrow.inner.entity(), 'year').replace('1999');
      await trx.commit();
    }

    const changes = await watcher.takeOne();
    expect(changes.length).toBe(4);
    expect(changes[0][0].equals(ids[4])).toBe(true);
    expect(changes[0][1]).toBe('Remove');
    expect(changes[1][0].equals(ids[6])).toBe(true);
    expect(changes[1][1]).toBe('Remove');
    expect(changes[2][0].equals(ids[3])).toBe(true);
    expect(changes[2][1]).toBe('Add');
    expect(changes[3][0].equals(ids[2])).toBe(true);
    expect(changes[3][1]).toBe('Add');

    expect(years(query)).toEqual(['2027', '2025', '2023', '2022']);
    expect(await watcher.quiesce()).toBe(0);

    conn.destroy();
  });
});
