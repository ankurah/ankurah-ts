// MIRRORS: ankurah/tests/tests/local_subscription.rs

import { describe, test, expect, afterEach } from 'bun:test';
import {
  EntityId,
  CollectionId,
} from '@ankurah/proto';
import { parseSelection } from '@ankurah/ankql';
import type { SubscriptionGuard } from '@ankurah/signals';

import { Node, matchArgs } from '../src/node.ts';
import { OpenPolicy } from '../src/policy.ts';
import { defineModel, lww, yrsText } from '../src/define-model.ts';
import { LiveQuery, EntityLiveQuery } from '../src/livequery.ts';
import type { ChangeSet, ChangeKind, ItemChange } from '../src/changes.ts';
import type { ViewInstance, ModelDefinition } from '../src/model.ts';
import { YjsBackend } from '../src/property/backend/yjs.ts';
import { LWWBackend } from '../src/property/backend/lww.ts';
import { YrsString } from '../src/property/value/yrs_string.ts';
import { MemoryStorageEngine } from '../../storage-memory/src/index.ts';

// ---------------------------------------------------------------------------
// Test model definitions
// ---------------------------------------------------------------------------

// Album model — matches Rust Album { name: String, year: String }
// In Rust: name is String (LWW), year is String (YrsString)
// Both name and year are YrsString in the Rust derive macro default for String fields.
// Check Rust common.rs: #[derive(Model)] pub struct Album { pub name: String, pub year: String }
// The Rust derive macro maps String -> YrsString as the active type.
const AlbumDef = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// Pet model — matches Rust Pet { name: String, age: String }
const PetDef = defineModel('pet', {
  name: yrsText(),
  age: yrsText(),
});

// ---------------------------------------------------------------------------
// TestWatcher — accumulates ChangeSet notifications
// ---------------------------------------------------------------------------

/**
 * TS equivalent of the Rust TestWatcher<ChangeSet<R>, Vec<(EntityId, ChangeKind)>>.
 * Accumulates batches of change notifications and provides drain/wait methods.
 */
class TestWatcher {
  private batches: Array<Array<[EntityId, ChangeKind]>> = [];
  private resolvers: Array<() => void> = [];

  /**
   * Returns a callback suitable for LiveQuery.subscribe().
   */
  listener(): (changeset: ChangeSet<ViewInstance>) => void {
    return (changeset: ChangeSet<ViewInstance>) => {
      const batch: Array<[EntityId, ChangeKind]> = changeset.changes.map(
        (change: ItemChange<ViewInstance>) => [change.item.id(), change.kind] as [EntityId, ChangeKind],
      );
      this.batches.push(batch);
      // Wake up any pending waiters
      for (const resolve of this.resolvers) {
        resolve();
      }
      this.resolvers = [];
    };
  }

  /**
   * Drain all accumulated batches and clear internal state.
   * Returns the accumulated batches.
   */
  drain(): Array<Array<[EntityId, ChangeKind]>> {
    const result = this.batches;
    this.batches = [];
    return result;
  }

  /**
   * Wait for the next batch (with timeout).
   * Returns the first pending batch or waits for one.
   */
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

  /**
   * Wait briefly (100ms) for any additional items, then return count of accumulated batches.
   * Useful for asserting quiescence (no unexpected notifications).
   */
  async quiesce(): Promise<number> {
    await new Promise((resolve) => setTimeout(resolve, 100));
    return this.batches.length;
  }

  /**
   * Current batch count without waiting.
   */
  count(): number {
    return this.batches.length;
  }
}

// ---------------------------------------------------------------------------
// Node setup helper
// ---------------------------------------------------------------------------

function createTestNode() {
  const engine = new MemoryStorageEngine();
  const node = new Node({
    storageEngine: engine,
    policyAgent: new OpenPolicy(),
    durable: true,
  });
  return node;
}

// ---------------------------------------------------------------------------
// queryWait helper — creates a LiveQuery and waits for initialization
// ---------------------------------------------------------------------------

async function queryWait(
  node: Node,
  model: ModelDefinition<ViewInstance>,
  predicateStr: string,
): Promise<LiveQuery<ViewInstance>> {
  const ctx = node.context();
  const selection = parseSelection(predicateStr);
  const args = matchArgs(selection, true);
  const lq = ctx.query(model, args);
  await lq.waitInitialized();
  return lq;
}

// ---------------------------------------------------------------------------
// Helper to get a YrsString handle for mutation
// ---------------------------------------------------------------------------

function getYrsStringHandle(entity: import('../src/entity.ts').Entity, fieldName: string): YrsString {
  const backend = entity.getBackend(YjsBackend);
  return new YrsString(fieldName, backend, entity);
}

// ---------------------------------------------------------------------------
// Cleanup tracking
// ---------------------------------------------------------------------------

const guards: Array<{ drop(): void }> = [];

afterEach(() => {
  for (const guard of guards) {
    if ('drop' in guard && typeof guard.drop === 'function') {
      guard.drop();
    }
  }
  guards.length = 0;
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('LiveQuery - basic_local_subscription', () => {
  test('entities matching predicate appear in LiveQuery, updates trigger add notifications', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Create some initial entities
    // Album years: 2008, 2013, 2016, 2024
    const trx1 = ctx.begin();

    const walkingBorrow = await trx1.create(AlbumDef, {});
    const walkingEntity = walkingBorrow.inner.entity();
    getYrsStringHandle(walkingEntity, 'name').insert(0, 'Walking on a Dream');
    getYrsStringHandle(walkingEntity, 'year').insert(0, '2008');

    const iceBorrow = await trx1.create(AlbumDef, {});
    const iceEntity = iceBorrow.inner.entity();
    getYrsStringHandle(iceEntity, 'name').insert(0, 'Ice on the Dune');
    getYrsStringHandle(iceEntity, 'year').insert(0, '2013');
    const iceId = iceBorrow.inner.id();

    const twoVinesBorrow = await trx1.create(AlbumDef, {});
    const twoVinesEntity = twoVinesBorrow.inner.entity();
    getYrsStringHandle(twoVinesEntity, 'name').insert(0, 'Two Vines');
    getYrsStringHandle(twoVinesEntity, 'year').insert(0, '2016');
    const twoVinesId = twoVinesBorrow.inner.id();

    const askThatGodBorrow = await trx1.create(AlbumDef, {});
    const askThatGodEntity = askThatGodBorrow.inner.entity();
    getYrsStringHandle(askThatGodEntity, 'name').insert(0, 'Ask That God');
    getYrsStringHandle(askThatGodEntity, 'year').insert(0, '2024');
    const askThatGodId = askThatGodBorrow.inner.id();

    await trx1.commit();

    // Set up LiveQuery with predicate year > '2015'
    const lq = await queryWait(node, AlbumDef, "year > '2015'");
    guards.push(lq);

    // Set up watcher
    const watcher = new TestWatcher();
    const subGuard = lq.subscribe(watcher.listener());
    guards.push(subGuard);

    // Initial state should have Two Vines (2016) and Ask That God (2024)
    const quiesceCount = await watcher.quiesce();
    expect(quiesceCount).toBe(0);

    const items = lq.peek();
    expect(items.length).toBe(2);

    // Verify the IDs (sorted for deterministic comparison)
    const actualIds = lq.idsSorted();
    const expectedIds = [twoVinesId, askThatGodId].sort((a, b) => {
      const aBytes = a.toBytes();
      const bBytes = b.toBytes();
      for (let i = 0; i < Math.min(aBytes.length, bBytes.length); i++) {
        if (aBytes[i] < bBytes[i]) return -1;
        if (aBytes[i] > bBytes[i]) return 1;
      }
      return aBytes.length - bBytes.length;
    });
    expect(actualIds.length).toBe(expectedIds.length);
    for (let i = 0; i < actualIds.length; i++) {
      expect(actualIds[i].equals(expectedIds[i])).toBe(true);
    }

    // Update Ice on the Dune's year from 2013 to 2020 (now matches year > '2015')
    const trx2 = ctx.begin();
    const iceBorrow2 = await trx2.get(AlbumDef, iceId);
    const iceEntity2 = iceBorrow2.inner.entity();
    // Overwrite: delete existing "2013" (4 chars) then insert "2020"
    getYrsStringHandle(iceEntity2, 'year').overwrite(0, 4, '2020');
    await trx2.commit();

    // Wait for the notification
    const notification = await watcher.takeOne();

    // Verify Ice on the Dune was added
    expect(notification.length).toBe(1);
    expect(notification[0][0].equals(iceId)).toBe(true);
    expect(notification[0][1]).toBe('Add');

    // After update, should have all three albums matching
    const itemsAfter = lq.peek();
    expect(itemsAfter.length).toBe(3);
  });
});

describe('LiveQuery - complex_local_subscription', () => {
  test('complex predicate with OR/AND tracks add/update/remove transitions', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Set up LiveQuery with complex predicate BEFORE creating entities
    // Predicate: name = 'Rex' OR (age > '2' and age < '5')
    // Note: In the Rust tests, age is a YrsString (text), so comparisons are string-based
    const lq = await queryWait(node, PetDef, "name = 'Rex' OR (age > '2' and age < '5')");
    guards.push(lq);

    const watcher = new TestWatcher();
    const subGuard = lq.subscribe(watcher.listener());
    guards.push(subGuard);

    // Create test entities
    const trx1 = ctx.begin();

    const rexBorrow = await trx1.create(PetDef, {});
    const rexEntity = rexBorrow.inner.entity();
    getYrsStringHandle(rexEntity, 'name').insert(0, 'Rex');
    getYrsStringHandle(rexEntity, 'age').insert(0, '1');
    const rexId = rexBorrow.inner.id();

    const snuffyBorrow = await trx1.create(PetDef, {});
    const snuffyEntity = snuffyBorrow.inner.entity();
    getYrsStringHandle(snuffyEntity, 'name').insert(0, 'Snuffy');
    getYrsStringHandle(snuffyEntity, 'age').insert(0, '2');
    const snuffyId = snuffyBorrow.inner.id();

    const jasperBorrow = await trx1.create(PetDef, {});
    const jasperEntity = jasperBorrow.inner.entity();
    getYrsStringHandle(jasperEntity, 'name').insert(0, 'Jasper');
    getYrsStringHandle(jasperEntity, 'age').insert(0, '6');
    const jasperId = jasperBorrow.inner.id();

    await trx1.commit();

    // Verify initial state: only Rex should match (name = 'Rex')
    // Snuffy (age=2) doesn't match (age > '2' is false for '2')
    // Jasper (age=6) doesn't match (age < '5' is false for '6')
    const initialNotification = await watcher.takeOne();
    expect(initialNotification.length).toBe(1);
    expect(initialNotification[0][0].equals(rexId)).toBe(true);
    expect(initialNotification[0][1]).toBe('Add');

    // Update Rex's age to 7 - should still match because name = 'Rex'
    {
      const trx = ctx.begin();
      const rexBorrow2 = await trx.get(PetDef, rexId);
      const rexEntity2 = rexBorrow2.inner.entity();
      getYrsStringHandle(rexEntity2, 'age').overwrite(0, 1, '7');
      await trx.commit();
    }

    // Verify Rex's update was received - should be Update since it still matches name = 'Rex'
    const rexUpdateNotification = await watcher.takeOne();
    expect(rexUpdateNotification.length).toBe(1);
    expect(rexUpdateNotification[0][0].equals(rexId)).toBe(true);
    expect(rexUpdateNotification[0][1]).toBe('Update');

    // Update Snuffy's age to 3 - now matches (age > '2' and age < '5')
    {
      const trx = ctx.begin();
      const snuffyBorrow2 = await trx.get(PetDef, snuffyId);
      const snuffyEntity2 = snuffyBorrow2.inner.entity();
      getYrsStringHandle(snuffyEntity2, 'age').overwrite(0, 1, '3');
      await trx.commit();
    }

    // Verify Snuffy's Add notification
    const snuffyAddNotification = await watcher.takeOne();
    expect(snuffyAddNotification.length).toBe(1);
    expect(snuffyAddNotification[0][0].equals(snuffyId)).toBe(true);
    expect(snuffyAddNotification[0][1]).toBe('Add');

    // Update Jasper's age to 4 - now matches (age > '2' and age < '5')
    {
      const trx = ctx.begin();
      const jasperBorrow2 = await trx.get(PetDef, jasperId);
      const jasperEntity2 = jasperBorrow2.inner.entity();
      getYrsStringHandle(jasperEntity2, 'age').overwrite(0, 1, '4');
      await trx.commit();
    }

    // Verify Jasper's Add notification
    const jasperAddNotification = await watcher.takeOne();
    expect(jasperAddNotification.length).toBe(1);
    expect(jasperAddNotification[0][0].equals(jasperId)).toBe(true);
    expect(jasperAddNotification[0][1]).toBe('Add');

    // Update Snuffy and Jasper to ages outside the range in a single transaction
    {
      const trx = ctx.begin();
      const snuffyBorrow3 = await trx.get(PetDef, snuffyId);
      const snuffyEntity3 = snuffyBorrow3.inner.entity();
      getYrsStringHandle(snuffyEntity3, 'age').overwrite(0, 1, '5');

      const jasperBorrow3 = await trx.get(PetDef, jasperId);
      const jasperEntity3 = jasperBorrow3.inner.entity();
      getYrsStringHandle(jasperEntity3, 'age').overwrite(0, 1, '6');

      await trx.commit();
    }

    // Verify both removals in a single batch
    const removalNotification = await watcher.takeOne();
    // Both Snuffy and Jasper should be removed in this single batch
    expect(removalNotification.length).toBe(2);

    // Sort by entity ID for deterministic comparison
    const sortedRemovals = [...removalNotification].sort((a, b) => {
      const aBytes = a[0].toBytes();
      const bBytes = b[0].toBytes();
      for (let i = 0; i < Math.min(aBytes.length, bBytes.length); i++) {
        if (aBytes[i] < bBytes[i]) return -1;
        if (aBytes[i] > bBytes[i]) return 1;
      }
      return aBytes.length - bBytes.length;
    });

    const expectedRemovals = [snuffyId, jasperId].sort((a, b) => {
      const aBytes = a.toBytes();
      const bBytes = b.toBytes();
      for (let i = 0; i < Math.min(aBytes.length, bBytes.length); i++) {
        if (aBytes[i] < bBytes[i]) return -1;
        if (aBytes[i] > bBytes[i]) return 1;
      }
      return aBytes.length - bBytes.length;
    });

    expect(sortedRemovals[0][0].equals(expectedRemovals[0])).toBe(true);
    expect(sortedRemovals[0][1]).toBe('Remove');
    expect(sortedRemovals[1][0].equals(expectedRemovals[1])).toBe(true);
    expect(sortedRemovals[1][1]).toBe('Remove');

    // Update Rex's name to no longer match (name != 'Rex')
    {
      const trx = ctx.begin();
      const rexBorrow3 = await trx.get(PetDef, rexId);
      const rexEntity3 = rexBorrow3.inner.entity();
      getYrsStringHandle(rexEntity3, 'name').overwrite(0, 3, 'NotRex');
      await trx.commit();
    }

    // Verify Rex's removal
    const rexRemovalNotification = await watcher.takeOne();
    expect(rexRemovalNotification.length).toBe(1);
    expect(rexRemovalNotification[0][0].equals(rexId)).toBe(true);
    expect(rexRemovalNotification[0][1]).toBe('Remove');
  });
});

describe('LiveQuery - resultset_vs_livequery_signal_semantics', () => {
  test('ResultSet Signal fires only on membership changes; LiveQuery Signal fires on all changes', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Create initial entities
    const trx1 = ctx.begin();

    const albumABorrow = await trx1.create(AlbumDef, {});
    const albumAEntity = albumABorrow.inner.entity();
    getYrsStringHandle(albumAEntity, 'name').insert(0, 'Album A');
    getYrsStringHandle(albumAEntity, 'year').insert(0, '2020');
    const albumAId = albumABorrow.inner.id();

    const albumBBorrow = await trx1.create(AlbumDef, {});
    const albumBEntity = albumBBorrow.inner.entity();
    getYrsStringHandle(albumBEntity, 'name').insert(0, 'Album B');
    getYrsStringHandle(albumBEntity, 'year').insert(0, '2015');
    const albumBId = albumBBorrow.inner.id();

    await trx1.commit();

    // Set up query for albums where year >= '2020'
    const lq = await queryWait(node, AlbumDef, "year >= '2020'");
    guards.push(lq);

    // Set up watchers for ResultSet Signal vs LiveQuery Signal
    let resultsetFireCount = 0;
    let livequeryFireCount = 0;

    const resultsetGuard = lq.inner.resultset.listen(() => {
      resultsetFireCount++;
    });

    const livequeryGuard = lq.listen(() => {
      livequeryFireCount++;
    });

    // Initial state - no notifications yet
    expect(resultsetFireCount).toBe(0);
    expect(livequeryFireCount).toBe(0);

    // Test 1: Property update (no membership change)
    // albumA is in the query, and stays in the query
    {
      const trx = ctx.begin();
      const albumABorrow2 = await trx.get(AlbumDef, albumAId);
      const albumAEntity2 = albumABorrow2.inner.entity();
      getYrsStringHandle(albumAEntity2, 'name').overwrite(0, 7, 'Album A Updated');
      await trx.commit();
    }

    // Wait for async processing to settle
    await new Promise((resolve) => setTimeout(resolve, 100));

    // ResultSet Signal should NOT fire (no membership change)
    // LiveQuery Signal SHOULD fire (entity property changed)
    expect(resultsetFireCount).toBe(0);
    expect(livequeryFireCount).toBe(1);

    // Reset counters for next test
    resultsetFireCount = 0;
    livequeryFireCount = 0;

    // Test 2: Entity enters the query (membership change)
    {
      const trx = ctx.begin();
      const albumBBorrow2 = await trx.get(AlbumDef, albumBId);
      const albumBEntity2 = albumBBorrow2.inner.entity();
      getYrsStringHandle(albumBEntity2, 'year').overwrite(0, 4, '2021');
      await trx.commit();
    }

    // Wait for async processing to settle
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Both signals should fire (membership changed)
    expect(resultsetFireCount).toBeGreaterThanOrEqual(1);
    expect(livequeryFireCount).toBeGreaterThanOrEqual(1);

    // Reset counters for next test
    resultsetFireCount = 0;
    livequeryFireCount = 0;

    // Test 3: Another property update (no membership change)
    {
      const trx = ctx.begin();
      const albumABorrow3 = await trx.get(AlbumDef, albumAId);
      const albumAEntity3 = albumABorrow3.inner.entity();
      getYrsStringHandle(albumAEntity3, 'name').overwrite(0, 15, 'Album A Changed Again');
      await trx.commit();
    }

    // Wait for async processing to settle
    await new Promise((resolve) => setTimeout(resolve, 100));

    // ResultSet Signal should NOT fire (no membership change)
    // LiveQuery Signal SHOULD fire (entity property changed)
    expect(resultsetFireCount).toBe(0);
    expect(livequeryFireCount).toBe(1);

    // Clean up listener guards
    resultsetGuard.drop();
    livequeryGuard.drop();
  });
});

// ---------------------------------------------------------------------------
// Helper: extract years from a LiveQuery's current items (ordered)
// Mirrors Rust fn years(query: &LiveQuery<AlbumView>) -> Vec<String>
// ---------------------------------------------------------------------------

function years(lq: LiveQuery<ViewInstance>): string[] {
  return lq.peek().map((item) => {
    // AlbumDef View has .year() accessor
    // The accessor returns either a plain string or a Value object { type: 'String', value: string }
    const raw = (item as any).year();
    if (typeof raw === 'string') return raw;
    if (raw && typeof raw === 'object' && 'value' in raw) return String(raw.value);
    return String(raw);
  });
}

// ---------------------------------------------------------------------------
// Helper: create N albums with sequential years, returns [EntityId, ...]
// Mirrors Rust fn create_albums(ctx, years) -> Vec<EntityId>
// ---------------------------------------------------------------------------

async function createAlbums(
  node: Node,
  yearStart: number,
  yearEnd: number,
): Promise<import('@ankurah/proto').EntityId[]> {
  const ctx = node.context();
  const trx = ctx.begin();
  const ids: import('@ankurah/proto').EntityId[] = [];

  for (let y = yearStart; y <= yearEnd; y++) {
    const borrow = await trx.create(AlbumDef, {});
    const entity = borrow.inner.entity();
    getYrsStringHandle(entity, 'name').insert(0, `Album ${y}`);
    getYrsStringHandle(entity, 'year').insert(0, String(y));
    ids.push(borrow.inner.id());
  }

  await trx.commit();
  return ids;
}

// ---------------------------------------------------------------------------
// Test 4: test_predicate_update (MIRRORS: ankurah/tests/tests/update_predicate.rs)
// ---------------------------------------------------------------------------

describe('LiveQuery - predicate_update', () => {
  test('updateSelectionWait changes the predicate and notifies watchers of membership changes', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Create 3 albums: Alpha (2020), Bravo (2021), Charlie (2022)
    const trx = ctx.begin();

    const alphaBorrow = await trx.create(AlbumDef, {});
    const alphaEntity = alphaBorrow.inner.entity();
    getYrsStringHandle(alphaEntity, 'name').insert(0, 'Alpha');
    getYrsStringHandle(alphaEntity, 'year').insert(0, '2020');
    const alphaId = alphaBorrow.inner.id();

    const bravoBorrow = await trx.create(AlbumDef, {});
    const bravoEntity = bravoBorrow.inner.entity();
    getYrsStringHandle(bravoEntity, 'name').insert(0, 'Bravo');
    getYrsStringHandle(bravoEntity, 'year').insert(0, '2021');
    const bravoId = bravoBorrow.inner.id();

    const charlieBorrow = await trx.create(AlbumDef, {});
    const charlieEntity = charlieBorrow.inner.entity();
    getYrsStringHandle(charlieEntity, 'name').insert(0, 'Charlie');
    getYrsStringHandle(charlieEntity, 'year').insert(0, '2022');
    const charlieId = charlieBorrow.inner.id();

    await trx.commit();

    // LiveQuery: year > '2020' -- should match Bravo + Charlie
    const lq = await queryWait(node, AlbumDef, "year > '2020'");
    guards.push(lq);

    const watcher = new TestWatcher();
    const subGuard = lq.subscribe(watcher.listener());
    guards.push(subGuard);

    // Verify initial state: Bravo + Charlie
    const expectedInitialIds = [bravoId, charlieId].sort((a, b) => {
      const aBytes = a.toBytes();
      const bBytes = b.toBytes();
      for (let i = 0; i < Math.min(aBytes.length, bBytes.length); i++) {
        if (aBytes[i] < bBytes[i]) return -1;
        if (aBytes[i] > bBytes[i]) return 1;
      }
      return aBytes.length - bBytes.length;
    });
    expect(lq.idsSorted().length).toBe(2);
    for (let i = 0; i < expectedInitialIds.length; i++) {
      expect(lq.idsSorted()[i].equals(expectedInitialIds[i])).toBe(true);
    }
    expect(await watcher.quiesce()).toBe(0); // no changes yet

    // Update predicate to be MORE restrictive: year > '2021' -- removes Bravo
    await lq.inner.updateSelectionWait("year > '2021'");

    // Verify only Charlie remains
    expect(lq.ids().length).toBe(1);
    expect(lq.ids()[0].equals(charlieId)).toBe(true);

    // Watcher should get Remove for Bravo
    const removeNotification = await watcher.takeOne();
    expect(removeNotification.length).toBe(1);
    expect(removeNotification[0][0].equals(bravoId)).toBe(true);
    expect(removeNotification[0][1]).toBe('Remove');

    // Update predicate to be LESS restrictive: year >= '2020' -- adds Alpha + Bravo
    await lq.inner.updateSelectionWait("year >= '2020'");

    // Verify all 3 albums are now in the result
    const expectedAllIds = [alphaId, bravoId, charlieId].sort((a, b) => {
      const aBytes = a.toBytes();
      const bBytes = b.toBytes();
      for (let i = 0; i < Math.min(aBytes.length, bBytes.length); i++) {
        if (aBytes[i] < bBytes[i]) return -1;
        if (aBytes[i] > bBytes[i]) return 1;
      }
      return aBytes.length - bBytes.length;
    });
    expect(lq.idsSorted().length).toBe(3);
    for (let i = 0; i < expectedAllIds.length; i++) {
      expect(lq.idsSorted()[i].equals(expectedAllIds[i])).toBe(true);
    }

    // Watcher should get Initial for Alpha + Bravo (sorted by entity ID for determinism)
    const addNotification = watcher.drain();
    expect(addNotification.length).toBe(1); // single batch
    const batch = addNotification[0];
    expect(batch.length).toBe(2);

    const sortedBatch = [...batch].sort((a, b) => {
      const aBytes = a[0].toBytes();
      const bBytes = b[0].toBytes();
      for (let i = 0; i < Math.min(aBytes.length, bBytes.length); i++) {
        if (aBytes[i] < bBytes[i]) return -1;
        if (aBytes[i] > bBytes[i]) return 1;
      }
      return aBytes.length - bBytes.length;
    });
    const expectedInitials = [alphaId, bravoId].sort((a, b) => {
      const aBytes = a.toBytes();
      const bBytes = b.toBytes();
      for (let i = 0; i < Math.min(aBytes.length, bBytes.length); i++) {
        if (aBytes[i] < bBytes[i]) return -1;
        if (aBytes[i] > bBytes[i]) return 1;
      }
      return aBytes.length - bBytes.length;
    });
    expect(sortedBatch[0][0].equals(expectedInitials[0])).toBe(true);
    expect(sortedBatch[0][1]).toBe('Initial');
    expect(sortedBatch[1][0].equals(expectedInitials[1])).toBe(true);
    expect(sortedBatch[1][1]).toBe('Initial');

    // Should have no more changes
    expect(await watcher.quiesce()).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Test 5: test_single_node_gap_filling (MIRRORS: ankurah/tests/tests/limit_gap_filling.rs)
// ---------------------------------------------------------------------------

describe('LiveQuery - single_node_gap_filling', () => {
  test('ORDER BY + LIMIT gap filling fetches next entity when one exits the query', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Create 5 albums with years 2020-2024
    const ids = await createAlbums(node, 2020, 2024);

    // LiveQuery: year >= '2020' ORDER BY year ASC LIMIT 3
    // Should initially contain [2020, 2021, 2022]
    const lq = await queryWait(node, AlbumDef, "year >= '2020' ORDER BY year ASC LIMIT 3");
    guards.push(lq);

    const watcher = new TestWatcher();
    const subGuard = lq.subscribe(watcher.listener());
    guards.push(subGuard);

    // Verify initial state
    expect(await watcher.quiesce()).toBe(0);
    expect(lq.peek().length).toBe(3);
    expect(years(lq)).toEqual(['2020', '2021', '2022']);

    // Update album with year 2021 to 1999 (exits the query: year < '2020')
    {
      const trx = ctx.begin();
      const albumBorrow = await trx.get(AlbumDef, ids[1]); // 2021
      const albumEntity = albumBorrow.inner.entity();
      getYrsStringHandle(albumEntity, 'year').overwrite(0, 4, '1999');
      await trx.commit();
    }

    // Wait for gap filling: Remove for 2021, Add for 2023
    const notification = await watcher.takeOne();
    expect(notification.length).toBe(2);
    expect(notification[0][0].equals(ids[1])).toBe(true); // 2021
    expect(notification[0][1]).toBe('Remove');
    expect(notification[1][0].equals(ids[3])).toBe(true); // 2023
    expect(notification[1][1]).toBe('Add');

    // Final state should be [2020, 2022, 2023]
    expect(years(lq)).toEqual(['2020', '2022', '2023']);
    expect(await watcher.quiesce()).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Test 6: test_single_node_multiple_gap_filling (MIRRORS: ankurah/tests/tests/limit_gap_filling.rs)
// ---------------------------------------------------------------------------

describe('LiveQuery - single_node_multiple_gap_filling', () => {
  test('ORDER BY + LIMIT gap filling fetches multiple entities when multiple exit the query', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Create 11 albums with years 2020-2030
    const ids = await createAlbums(node, 2020, 2030);

    // LiveQuery: year >= '2020' ORDER BY year ASC LIMIT 5
    // Should initially contain [2020, 2021, 2022, 2023, 2024]
    const lq = await queryWait(node, AlbumDef, "year >= '2020' ORDER BY year ASC LIMIT 5");
    guards.push(lq);

    const watcher = new TestWatcher();
    const subGuard = lq.subscribe(watcher.listener());
    guards.push(subGuard);

    // Verify initial state
    expect(await watcher.quiesce()).toBe(0);
    expect(years(lq)).toEqual(['2020', '2021', '2022', '2023', '2024']);

    // Remove 2 albums (2021 and 2023) in same transaction
    {
      const trx = ctx.begin();
      const album2021Borrow = await trx.get(AlbumDef, ids[1]); // 2021
      const album2021Entity = album2021Borrow.inner.entity();
      getYrsStringHandle(album2021Entity, 'year').overwrite(0, 4, '1999');

      const album2023Borrow = await trx.get(AlbumDef, ids[3]); // 2023
      const album2023Entity = album2023Borrow.inner.entity();
      getYrsStringHandle(album2023Entity, 'year').overwrite(0, 4, '1999');

      await trx.commit();
    }

    // Wait for consolidated gap filling: 2 removes + 2 adds
    const notification = await watcher.takeOne();
    expect(notification.length).toBe(4);

    // Verify the changes: Remove for 2021, Remove for 2023, Add for 2025, Add for 2026
    expect(notification[0][0].equals(ids[1])).toBe(true); // 2021
    expect(notification[0][1]).toBe('Remove');
    expect(notification[1][0].equals(ids[3])).toBe(true); // 2023
    expect(notification[1][1]).toBe('Remove');
    expect(notification[2][0].equals(ids[5])).toBe(true); // 2025
    expect(notification[2][1]).toBe('Add');
    expect(notification[3][0].equals(ids[6])).toBe(true); // 2026
    expect(notification[3][1]).toBe('Add');

    // Final state: [2020, 2022, 2024, 2025, 2026]
    expect(years(lq)).toEqual(['2020', '2022', '2024', '2025', '2026']);
    expect(await watcher.quiesce()).toBe(0);
  });
});
