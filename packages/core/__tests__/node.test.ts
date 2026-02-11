// MIRRORS: ankurah/core/src/node.rs (tests)
// Tests for Node, NodeAndContext, Context integration, StorageEngine, PolicyAgent

import { describe, expect, test } from 'bun:test';
import {
  EntityId,
  Clock,
  Event,
  Operation,
  OperationSet,
  Attested,
  EntityState,
  State,
  StateBuffers,
} from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';

import { Entity, WeakEntitySet } from '../src/entity.ts';
import { Node, NodeAndContext, matchArgs } from '../src/node.ts';
import { Context } from '../src/context.ts';
import { Transaction } from '../src/transaction.ts';
import { OpenPolicy } from '../src/policy.ts';
import type { StorageEngine, StorageCollection } from '../src/storage.ts';
import { defineModel, lww, yrsText } from '../src/define-model.ts';
import { LWWBackend } from '../src/property/backend/lww.ts';

// ── Test model ──

const TestModel = defineModel('test_node', {
  name: lww<string>(),
  count: lww<number>(),
  bio: yrsText(),
});

// ── Mock StorageCollection ──

class MockStorageCollection implements StorageCollection {
  readonly states = new Map<string, Attested<EntityState>>();
  readonly events: Attested<Event>[] = [];

  async getState(id: EntityId): Promise<Attested<EntityState>> {
    const state = this.states.get(id.toString());
    if (!state) {
      throw new Error(`Entity not found: ${id}`);
    }
    return state;
  }

  async setState(state: Attested<EntityState>): Promise<void> {
    this.states.set(state.payload.entityId.toString(), state);
  }

  async addEvent(event: Attested<Event>): Promise<void> {
    this.events.push(event);
  }

  async fetchStates(_selection: Selection): Promise<Attested<EntityState>[]> {
    return [...this.states.values()];
  }
}

// ── Mock StorageEngine ──

class MockStorageEngine implements StorageEngine {
  readonly collections = new Map<string, MockStorageCollection>();

  async collection(id: string): Promise<StorageCollection> {
    let col = this.collections.get(id);
    if (!col) {
      col = new MockStorageCollection();
      this.collections.set(id, col);
    }
    return col;
  }
}

// ── Helper ──

function createNode(opts?: { durable?: boolean }): { node: Node; storage: MockStorageEngine } {
  const storage = new MockStorageEngine();
  const node = new Node({
    storageEngine: storage,
    policyAgent: new OpenPolicy(),
    durable: opts?.durable ?? false,
  });
  return { node, storage };
}

// ── MatchArgs Tests ──

describe('matchArgs', () => {
  test('creates MatchArgs with default cached=true', () => {
    const selection = { predicate: { type: 'True' } } as unknown as Selection;
    const args = matchArgs(selection);
    expect(args.selection).toBe(selection);
    expect(args.cached).toBe(true);
  });

  test('creates MatchArgs with cached=false', () => {
    const selection = { predicate: { type: 'True' } } as unknown as Selection;
    const args = matchArgs(selection, false);
    expect(args.cached).toBe(false);
  });
});

// ── Node Tests ──

describe('Node', () => {
  test('constructor generates unique ID', () => {
    const { node: node1 } = createNode();
    const { node: node2 } = createNode();
    expect(node1.id.equals(node2.id)).toBe(false);
  });

  test('constructor uses provided ID', () => {
    const id = EntityId.new();
    const storage = new MockStorageEngine();
    const node = new Node({
      id,
      storageEngine: storage,
      policyAgent: new OpenPolicy(),
    });
    expect(node.id.equals(id)).toBe(true);
  });

  test('durable defaults to false', () => {
    const { node } = createNode();
    expect(node.durable).toBe(false);
  });

  test('durable can be set to true', () => {
    const { node } = createNode({ durable: true });
    expect(node.durable).toBe(true);
  });

  test('entities starts empty', () => {
    const { node } = createNode();
    const randomId = EntityId.new();
    expect(node.entities.get(randomId)).toBeNull();
  });

  test('toString() includes short ID', () => {
    const { node } = createNode();
    const str = node.toString();
    expect(str).toContain('Node(');
    expect(str).toContain(')');
  });

  test('context() creates a Context', () => {
    const { node } = createNode();
    const ctx = node.context();
    expect(ctx).toBeInstanceOf(Context);
  });

  test('context() with context data', () => {
    const storage = new MockStorageEngine();
    const node = new Node({
      storageEngine: storage,
      policyAgent: new OpenPolicy(),
      contextData: { userId: 'test-user' },
    });
    const ctx = node.context();
    expect(ctx).toBeInstanceOf(Context);
  });

  test('context() nodeId matches node ID', () => {
    const { node } = createNode();
    const ctx = node.context();
    expect(ctx.nodeId().equals(node.id)).toBe(true);
  });

  test('fetchEntitiesFromLocal returns empty for empty storage', async () => {
    const { node } = createNode();
    const selection = { predicate: { type: 'True' } } as unknown as Selection;
    const entities = await node.fetchEntitiesFromLocal('test_node', selection);
    expect(entities).toHaveLength(0);
  });

  test('fetchEntitiesFromLocal returns entities from storage', async () => {
    const { node, storage } = createNode();

    // Seed the storage with an entity state
    const entityId = EntityId.new();
    const stateBuffers = new StateBuffers(new Map());
    const state = new State(stateBuffers, Clock.empty());
    const entityState = new EntityState(entityId, 'test_node' as any, state);
    const attested = new Attested(entityState);

    const col = await storage.collection('test_node') as MockStorageCollection;
    col.states.set(entityId.toString(), attested);

    const selection = { predicate: { type: 'True' } } as unknown as Selection;
    const entities = await node.fetchEntitiesFromLocal('test_node', selection);
    expect(entities).toHaveLength(1);
    expect(entities[0].id().equals(entityId)).toBe(true);
  });
});

// ── NodeAndContext Tests ──

describe('NodeAndContext', () => {
  test('nodeId() returns the node ID', () => {
    const { node } = createNode();
    const nac = new NodeAndContext(node, null);
    expect(nac.nodeId().equals(node.id)).toBe(true);
  });

  test('createEntity() creates a transacted entity', () => {
    const { node } = createNode();
    const nac = new NodeAndContext(node, null);
    const alive = { value: true };
    const entity = nac.createEntity('test_node' as any, alive);

    expect(entity.kind.type).toBe('Transacted');
    expect(entity.isWritable()).toBe(true);
    expect(entity.collection()).toBe('test_node');
  });

  test('createEntity() registers primary in WeakEntitySet', () => {
    const { node } = createNode();
    const nac = new NodeAndContext(node, null);
    const alive = { value: true };
    const entity = nac.createEntity('test_node' as any, alive);

    // The primary entity should be in the weak entity set
    const primary = node.entities.get(entity.id());
    expect(primary).not.toBeNull();
    expect(primary!.kind.type).toBe('Primary');
  });

  test('checkWrite() succeeds with OpenPolicy', () => {
    const { node } = createNode();
    const nac = new NodeAndContext(node, null);
    const entity = Entity.create(EntityId.new(), 'test_node' as any);
    // Should not throw
    nac.checkWrite(entity);
  });

  test('getEntity() retrieves from storage', async () => {
    const { node, storage } = createNode();
    const nac = new NodeAndContext(node, null);

    // Seed storage
    const entityId = EntityId.new();
    const stateBuffers = new StateBuffers(new Map());
    const state = new State(stateBuffers, Clock.empty());
    const entityState = new EntityState(entityId, 'test_node' as any, state);
    const col = await storage.collection('test_node') as MockStorageCollection;
    col.states.set(entityId.toString(), new Attested(entityState));

    const entity = await nac.getEntity(entityId, 'test_node' as any, false);
    expect(entity.id().equals(entityId)).toBe(true);
  });

  test('getEntity() returns resident entity if available', async () => {
    const { node } = createNode();
    const nac = new NodeAndContext(node, null);

    // Create an entity and register it
    const id = EntityId.new();
    const entity = Entity.create(id, 'test_node' as any);
    node.entities.register(entity);

    const retrieved = await nac.getEntity(id, 'test_node' as any, true);
    expect(retrieved).toBe(entity);
  });

  test('getEntity() throws for non-existent entity', async () => {
    const { node } = createNode();
    const nac = new NodeAndContext(node, null);
    const id = EntityId.new();

    await expect(nac.getEntity(id, 'test_node' as any, false)).rejects.toThrow();
  });

  test('getResidentEntity() returns null if not resident', () => {
    const { node } = createNode();
    const nac = new NodeAndContext(node, null);
    expect(nac.getResidentEntity(EntityId.new())).toBeNull();
  });

  test('getResidentEntity() returns entity if resident', () => {
    const { node } = createNode();
    const nac = new NodeAndContext(node, null);

    const id = EntityId.new();
    const entity = Entity.create(id, 'test_node' as any);
    node.entities.register(entity);

    expect(nac.getResidentEntity(id)).toBe(entity);
  });

  test('fetchEntities() delegates to node', async () => {
    const { node, storage } = createNode();
    const nac = new NodeAndContext(node, null);

    // Seed storage
    const entityId = EntityId.new();
    const stateBuffers = new StateBuffers(new Map());
    const state = new State(stateBuffers, Clock.empty());
    const entityState = new EntityState(entityId, 'test_node' as any, state);
    const col = await storage.collection('test_node') as MockStorageCollection;
    col.states.set(entityId.toString(), new Attested(entityState));

    const selection = { predicate: { type: 'True' } } as unknown as Selection;
    const entities = await nac.fetchEntities('test_node' as any, matchArgs(selection));
    expect(entities).toHaveLength(1);
  });
});

// ── Full Integration: Node → Context → Transaction → Commit ──

describe('Node integration', () => {
  test('create and commit entity through full pipeline', async () => {
    const { node, storage } = createNode();
    const ctx = node.context();
    const trx = ctx.begin();

    const borrow = await trx.create(TestModel, { name: 'Alice', count: 42 });
    const entityId = borrow.inner.id();

    await trx.commit();

    // After commit, entity should be non-writable
    expect(borrow.inner.entity().isWritable()).toBe(false);

    // Entity should be in storage
    const col = storage.collections.get('test_node');
    expect(col).toBeDefined();
    expect(col!.states.size).toBeGreaterThanOrEqual(1);

    // Events should have been recorded
    expect(col!.events.length).toBeGreaterThanOrEqual(1);
  });

  test('create multiple entities in one transaction', async () => {
    const { node, storage } = createNode();
    const ctx = node.context();
    const trx = ctx.begin();

    await trx.create(TestModel, { name: 'Alice', count: 1 });
    await trx.create(TestModel, { name: 'Bob', count: 2 });
    await trx.create(TestModel, { name: 'Charlie', count: 3 });

    await trx.commit();

    const col = storage.collections.get('test_node');
    expect(col).toBeDefined();
    expect(col!.states.size).toBe(3);
    expect(col!.events.length).toBe(3);
  });

  test('edit existing entity and commit', async () => {
    const { node, storage } = createNode();
    const ctx = node.context();

    // First transaction: create entity
    const trx1 = ctx.begin();
    const borrow1 = await trx1.create(TestModel, { name: 'Alice', count: 1 });
    const entityId = borrow1.inner.id();
    await trx1.commit();

    // Second transaction: edit entity
    const trx2 = ctx.begin();
    const borrow2 = await trx2.get(TestModel, entityId);
    const lwwBackend = borrow2.inner.entity().getBackend(LWWBackend);
    lwwBackend.set('name', { type: 'String', value: 'Alice Updated' });
    await trx2.commit();

    // Should have 2 events total
    const col = storage.collections.get('test_node');
    expect(col).toBeDefined();
    expect(col!.events.length).toBe(2);
  });

  test('rollback does not persist', async () => {
    const { node, storage } = createNode();
    const ctx = node.context();
    const trx = ctx.begin();

    await trx.create(TestModel, { name: 'Alice', count: 1 });
    trx.rollback();

    // Nothing should be stored
    const col = storage.collections.get('test_node');
    // Collection may or may not exist, but no states should be saved
    if (col) {
      expect(col.states.size).toBe(0);
      expect(col.events.length).toBe(0);
    }
  });

  test('double commit is rejected', async () => {
    const { node } = createNode();
    const ctx = node.context();
    const trx = ctx.begin();

    await trx.create(TestModel, { name: 'Test' });
    await trx.commit();

    await expect(trx.commit()).rejects.toThrow('already committed');
  });

  test('commit empty transaction succeeds', async () => {
    const { node } = createNode();
    const ctx = node.context();
    const trx = ctx.begin();

    // No entities created — nothing to commit
    await trx.commit();
    // Should complete without error
  });

  test('commit applies state to upstream entity', async () => {
    const { node } = createNode();
    const ctx = node.context();
    const trx = ctx.begin();

    const borrow = await trx.create(TestModel, { name: 'Alice', count: 42 });
    const entityId = borrow.inner.id();

    await trx.commit();

    // The canonical (upstream) entity should have the committed state
    const canonical = node.entities.get(entityId);
    expect(canonical).not.toBeNull();
    expect(canonical!.kind.type).toBe('Primary');

    // Check that the LWW backend on the canonical entity has the value
    const lww = canonical!.getBackend(LWWBackend);
    const nameVal = lww.get('name');
    expect(nameVal).not.toBeNull();
    expect((nameVal as any).value).toBe('Alice');
  });

  test('independent transactions are isolated', async () => {
    const { node } = createNode();
    const ctx = node.context();

    // Create an entity first
    const trx1 = ctx.begin();
    const borrow1 = await trx1.create(TestModel, { name: 'Original', count: 0 });
    const entityId = borrow1.inner.id();
    await trx1.commit();

    // Two independent transactions editing the same entity
    const trx2 = ctx.begin();
    const borrow2 = await trx2.get(TestModel, entityId);
    const lww2 = borrow2.inner.entity().getBackend(LWWBackend);
    lww2.set('name', { type: 'String', value: 'From Trx2' });

    const trx3 = ctx.begin();
    const borrow3 = await trx3.get(TestModel, entityId);
    const lww3 = borrow3.inner.entity().getBackend(LWWBackend);
    lww3.set('name', { type: 'String', value: 'From Trx3' });

    // Modifications are isolated
    expect((lww2.get('name') as any).value).toBe('From Trx2');
    expect((lww3.get('name') as any).value).toBe('From Trx3');

    // Commit trx2 first
    await trx2.commit();

    const canonical = node.entities.get(entityId);
    expect((canonical!.getBackend(LWWBackend).get('name') as any).value).toBe('From Trx2');

    // trx3 still has its own value
    expect((lww3.get('name') as any).value).toBe('From Trx3');
  });

  test('context creates multiple independent transactions', () => {
    const { node } = createNode();
    const ctx = node.context();

    const trx1 = ctx.begin();
    const trx2 = ctx.begin();

    expect(trx1).not.toBe(trx2);
    expect(trx1.id.equals(trx2.id)).toBe(false);
    expect(trx1.alive).not.toBe(trx2.alive);
  });
});

// ── OpenPolicy Tests ──

describe('OpenPolicy', () => {
  const policy = new OpenPolicy();

  test('checkWrite allows everything', () => {
    const entity = Entity.create(EntityId.new(), 'test' as any);
    // Should not throw
    policy.checkWrite(null, entity, null);
  });

  test('canAccessCollection allows everything', () => {
    // Should not throw
    policy.canAccessCollection(null, 'any_collection' as any);
  });

  test('checkEvent returns null attestation', () => {
    const entity = Entity.create(EntityId.new(), 'test' as any);
    const ops = new OperationSet(new Map());
    const event = new Event('test' as any, entity.id(), ops, Clock.empty());
    const result = policy.checkEvent(null, entity, entity, event);
    expect(result).toBeNull();
  });

  test('attestState returns null', () => {
    const entityId = EntityId.new();
    const state = new State(new StateBuffers(new Map()), Clock.empty());
    const entityState = new EntityState(entityId, 'test' as any, state);
    const result = policy.attestState(entityState);
    expect(result).toBeNull();
  });
});
