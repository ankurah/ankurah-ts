// MIRRORS: ankurah/tests/tests/update_predicate.rs

import { describe, test, expect, afterEach } from 'bun:test';
import { EntityId } from '@ankurah/proto';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { LocalProcessConnection } from '@ankurah/connector-local';

import { Node, matchArgs, nocache } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import type { ChangeSet, ChangeKind, ItemChange } from '../../src/changes.ts';
import type { ViewInstance } from '../../src/model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';
import { YrsString } from '../../src/property/value/yrs_string.ts';
import type { Entity } from '../../src/entity.ts';

// ── Model ──
// Mirrors: common.rs `struct Album { pub name: String, pub year: String }`
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// ── TestWatcher ──
// Mirrors: common.rs TestWatcher<ChangeSet<R>, Vec<(EntityId, ChangeKind)>>
// Divergence: Uses Promise-based waiting instead of tokio::sync::Notify [E8].

class TestWatcher {
  private batches: Array<Array<[EntityId, ChangeKind]>> = [];
  private resolvers: Array<() => void> = [];

  listener(): (changeset: ChangeSet<ViewInstance>) => void {
    return (changeset: ChangeSet<ViewInstance>) => {
      const batch: Array<[EntityId, ChangeKind]> = changeset.changes.map(
        (change: ItemChange<ViewInstance>) => [change.item.id(), change.kind] as [EntityId, ChangeKind],
      );
      this.batches.push(batch);
      for (const resolve of this.resolvers) {
        resolve();
      }
      this.resolvers = [];
    };
  }

  drain(): Array<Array<[EntityId, ChangeKind]>> {
    const result = this.batches;
    this.batches = [];
    return result;
  }

  drainSorted(): Array<Array<[EntityId, ChangeKind]>> {
    const result = this.drain();
    for (const batch of result) {
      batch.sort((a, b) => compareEntityIds(a[0], b[0]));
    }
    return result;
  }

  async takeOne(timeoutMs = 5000): Promise<Array<[EntityId, ChangeKind]>> {
    if (this.batches.length > 0) {
      return this.batches.shift()!;
    }
    return new Promise<Array<[EntityId, ChangeKind]>>((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error(`takeOne() timed out after ${timeoutMs}ms`));
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

  count(): number {
    return this.batches.length;
  }
}

// ── Helpers ──

function compareEntityIds(a: EntityId, b: EntityId): number {
  const aBytes = a.toBytes();
  const bBytes = b.toBytes();
  const len = Math.min(aBytes.length, bBytes.length);
  for (let i = 0; i < len; i++) {
    if (aBytes[i] < bBytes[i]) return -1;
    if (aBytes[i] > bBytes[i]) return 1;
  }
  return aBytes.length - bBytes.length;
}

function sortedIds(...ids: EntityId[]): EntityId[] {
  return [...ids].sort(compareEntityIds);
}

function createTestNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
}

function getYrsStringHandle(entity: Entity, fieldName: string): YrsString {
  const backend = entity.getBackend(YjsBackend);
  return new YrsString(fieldName, backend, entity);
}

// ── Cleanup ──

const guards: Array<{ drop(): void }> = [];

afterEach(() => {
  for (const guard of guards) {
    if ('drop' in guard && typeof guard.drop === 'function') {
      guard.drop();
    }
  }
  guards.length = 0;
});

// ── Tests ──

// Mirrors: test_predicate_update
describe('test_predicate_update', () => {
  test('update_selection changes LiveQuery membership and fires appropriate notifications', async () => {
    // Rust: let storage_engine = SledStorageEngine::new_test()?;
    // Rust: let node = Node::new_durable(Arc::new(storage_engine), PermissiveAgent::new());
    // Divergence: MemoryStorageEngine instead of SledStorageEngine [E5]
    const node = createTestNode();
    // Rust: node.system.create().await?;
    // Divergence: SystemManager not yet ported — skip [E8]

    // Rust: let context = node.context_async(DEFAULT_CONTEXT).await;
    const context = node.context();

    // Create some test albums
    const trx = context.begin();

    const alphaBorrow = await trx.create(Album, {});
    const alphaEntity = alphaBorrow.inner.entity();
    getYrsStringHandle(alphaEntity, 'name').insert(0, 'Alpha');
    getYrsStringHandle(alphaEntity, 'year').insert(0, '2020');
    const aId = alphaBorrow.inner.id();

    const bravoBorrow = await trx.create(Album, {});
    const bravoEntity = bravoBorrow.inner.entity();
    getYrsStringHandle(bravoEntity, 'name').insert(0, 'Bravo');
    getYrsStringHandle(bravoEntity, 'year').insert(0, '2021');
    const bId = bravoBorrow.inner.id();

    const charlieBorrow = await trx.create(Album, {});
    const charlieEntity = charlieBorrow.inner.entity();
    getYrsStringHandle(charlieEntity, 'name').insert(0, 'Charlie');
    getYrsStringHandle(charlieEntity, 'year').insert(0, '2022');
    const cId = charlieBorrow.inner.id();

    await trx.commit();

    // Rust: let albums = context.query_wait::<AlbumView>("year > 2020").await?;
    const args = matchArgs("year > '2020'", true);
    const albums = context.query(Album, args);
    await albums.waitInitialized();
    guards.push(albums);

    const watcher = new TestWatcher();
    const subGuard = albums.subscribe(watcher.listener());
    guards.push(subGuard);

    // Should have Bravo, Charlie (sort for deterministic order)
    // Rust: assert_eq!(albums.ids_sorted(), sorted![b_id, c_id]);
    const initialIds = albums.idsSorted();
    const expectedInitial = sortedIds(bId, cId);
    expect(initialIds.length).toBe(expectedInitial.length);
    for (let i = 0; i < initialIds.length; i++) {
      expect(initialIds[i].equals(expectedInitial[i])).toBe(true);
    }

    // Rust: assert_eq!(watcher.quiesce().await, 0);
    expect(await watcher.quiesce()).toBe(0);

    // Update the predicate to be more restrictive: year > 2021 - Should remove Bravo
    // Rust: albums.update_selection_wait("year > 2021").await?;
    await albums.inner.updateSelectionWait("year > '2021'");

    // Rust: assert_eq!(albums.ids(), vec![c_id]);
    const afterNarrow = albums.ids();
    expect(afterNarrow.length).toBe(1);
    expect(afterNarrow[0].equals(cId)).toBe(true);

    // Rust: assert_eq!(watcher.take_one().await, vec![(b_id, ChangeKind::Remove)]);
    const removeNotification = await watcher.takeOne();
    expect(removeNotification.length).toBe(1);
    expect(removeNotification[0][0].equals(bId)).toBe(true);
    expect(removeNotification[0][1]).toBe('Remove');

    // Update predicate to be less restrictive: year >= "2020"
    // Rust: albums.update_selection_wait("year >= 2020").await?;
    await albums.inner.updateSelectionWait("year >= '2020'");

    // Should now have all 3 albums
    // Rust: assert_eq!(albums.ids_sorted(), sorted![a_id, b_id, c_id]);
    const afterWiden = albums.idsSorted();
    const expectedWiden = sortedIds(aId, bId, cId);
    expect(afterWiden.length).toBe(expectedWiden.length);
    for (let i = 0; i < afterWiden.length; i++) {
      expect(afterWiden[i].equals(expectedWiden[i])).toBe(true);
    }

    // Rust: assert_eq!(watcher.drain_sorted(), vec![sortby_t0![(a_id, ChangeKind::Initial), (b_id, ChangeKind::Initial)]]);
    const drainedBatches = watcher.drainSorted();
    expect(drainedBatches.length).toBe(1);
    const sortedBatch = drainedBatches[0];
    const expectedBatch = sortedIds(aId, bId);
    expect(sortedBatch.length).toBe(2);
    expect(sortedBatch[0][0].equals(expectedBatch[0])).toBe(true);
    expect(sortedBatch[0][1]).toBe('Initial');
    expect(sortedBatch[1][0].equals(expectedBatch[1])).toBe(true);
    expect(sortedBatch[1][1]).toBe('Initial');

    // should have no more changes
    // Rust: assert_eq!(watcher.quiesce().await, 0);
    expect(await watcher.quiesce()).toBe(0);
  });
});

// Mirrors: test_predicate_update_inter_node
describe('test_predicate_update_inter_node', () => {
  test('update_selection changes LiveQuery membership across nodes', async () => {
    // Create server (durable) and client (ephemeral) nodes
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

    // Create some test albums on the server
    let aId: EntityId, bId: EntityId, cId: EntityId;
    {
      const trx = serverCtx.begin();

      const alphaBorrow = await trx.create(Album, {});
      const alphaEntity = alphaBorrow.inner.entity();
      getYrsStringHandle(alphaEntity, 'name').insert(0, 'Alpha');
      getYrsStringHandle(alphaEntity, 'year').insert(0, '2020');
      aId = alphaBorrow.inner.id();

      const bravoBorrow = await trx.create(Album, {});
      const bravoEntity = bravoBorrow.inner.entity();
      getYrsStringHandle(bravoEntity, 'name').insert(0, 'Bravo');
      getYrsStringHandle(bravoEntity, 'year').insert(0, '2021');
      bId = bravoBorrow.inner.id();

      const charlieBorrow = await trx.create(Album, {});
      const charlieEntity = charlieBorrow.inner.entity();
      getYrsStringHandle(charlieEntity, 'name').insert(0, 'Charlie');
      getYrsStringHandle(charlieEntity, 'year').insert(0, '2022');
      cId = charlieBorrow.inner.id();

      await trx.commit();
    }

    // Create LiveQuery on client with initial predicate
    const albums = clientCtx.query(Album, nocache("year > '2020'"));
    await albums.waitInitialized();
    guards.push(albums);

    const watcher = new TestWatcher();
    const subGuard = albums.subscribe(watcher.listener());
    guards.push(subGuard);

    // Should have Bravo, Charlie (sort for deterministic order)
    const initialIds = albums.idsSorted();
    const expectedInitial = sortedIds(bId!, cId!);
    expect(initialIds.length).toBe(expectedInitial.length);
    for (let i = 0; i < initialIds.length; i++) {
      expect(initialIds[i].equals(expectedInitial[i])).toBe(true);
    }
    expect(await watcher.quiesce()).toBe(0);

    // Update the predicate to be more restrictive: year > 2021 - Should remove Bravo
    await albums.inner.updateSelectionWait("year > '2021'");

    const afterNarrow = albums.ids();
    expect(afterNarrow.length).toBe(1);
    expect(afterNarrow[0].equals(cId!)).toBe(true);

    const removeNotification = await watcher.takeOne();
    expect(removeNotification.length).toBe(1);
    expect(removeNotification[0][0].equals(bId!)).toBe(true);
    expect(removeNotification[0][1]).toBe('Remove');

    // Update predicate to be less restrictive: year >= "2020"
    await albums.inner.updateSelectionWait("year >= '2020'");

    // Should now have all 3 albums
    const afterWiden = albums.idsSorted();
    const expectedWiden = sortedIds(aId!, bId!, cId!);
    expect(afterWiden.length).toBe(expectedWiden.length);
    for (let i = 0; i < afterWiden.length; i++) {
      expect(afterWiden[i].equals(expectedWiden[i])).toBe(true);
    }

    const drainedBatches = watcher.drainSorted();
    expect(drainedBatches.length).toBe(1);
    const sortedBatch = drainedBatches[0];
    const expectedBatch = sortedIds(aId!, bId!);
    expect(sortedBatch.length).toBe(2);
    expect(sortedBatch[0][0].equals(expectedBatch[0])).toBe(true);
    expect(sortedBatch[0][1]).toBe('Initial');
    expect(sortedBatch[1][0].equals(expectedBatch[1])).toBe(true);
    expect(sortedBatch[1][1]).toBe('Initial');

    // should have no more changes
    expect(await watcher.quiesce()).toBe(0);

    conn.destroy();
  });
});
