// MIRRORS: ankurah/core/src/transaction.rs (tests)
// Tests for Transaction, Context, TContext, EntityChange, ItemChange

import { describe, expect, test } from 'bun:test';
import {
  EntityId,
  Clock,
  Event,
  Operation,
  OperationSet,
  Attested,
  TransactionId,
} from '@ankurah/proto';

import { Entity } from '../src/entity.ts';
import { Transaction } from '../src/transaction.ts';
import { Context, type TContext } from '../src/context.ts';
import { EntityChange, type ItemChange, itemChangeItem, itemChangeEvents, itemChangeKind } from '../src/changes.ts';
import { MutableBorrow } from '../src/model.ts';
import { defineModel, lww, yrsText } from '../src/define-model.ts';
import { LWWBackend } from '../src/property/backend/lww.ts';
import { YjsBackend } from '../src/property/backend/yjs.ts';

// ── Test model ──

const TestModel = defineModel('test_trx', {
  name: lww<string>(),
  count: lww<number>(),
  bio: yrsText(),
});

// ── Mock TContext ──

/**
 * A mock TContext for testing Transaction in isolation from Node/Storage.
 * Implements the minimum needed for create/get/edit/commit/rollback tests.
 */
class MockContext implements TContext {
  readonly entities = new Map<string, Entity>();
  commitCalled = false;
  lastCommittedTrx: Transaction | null = null;
  commitShouldFail = false;

  nodeId(): EntityId {
    return EntityId.new();
  }

  createEntity(collection: any, trxAlive: { value: boolean }): Entity {
    const id = EntityId.new();
    const primary = Entity.create(id, collection);
    this.entities.set(id.toString(), primary);
    return primary.snapshot(trxAlive);
  }

  checkWrite(_entity: Entity): void {
    // No-op: allow all writes in tests
  }

  async getEntity(id: EntityId, collection: any, _cached: boolean): Promise<Entity> {
    const entity = this.entities.get(id.toString());
    if (!entity) {
      throw new Error(`Entity not found: ${id}`);
    }
    return entity;
  }

  getResidentEntity(id: EntityId): Entity | null {
    return this.entities.get(id.toString()) ?? null;
  }

  async fetchEntities(_collection: any, _args: any): Promise<Entity[]> {
    return [...this.entities.values()];
  }

  query(_collectionId: any, _args: unknown): never {
    throw new Error('not implemented');
  }

  async collection(_id: any): Promise<any> {
    throw new Error('not implemented');
  }

  async commitLocalTrx(trx: Transaction): Promise<void> {
    if (this.commitShouldFail) {
      throw new Error('Commit failed');
    }

    // Simplified commit: mark alive as false, generate events, apply to upstream
    if (!trx.alive.value) {
      throw new Error('Transaction already committed or rolled back');
    }
    trx.alive.value = false;

    this.commitCalled = true;
    this.lastCommittedTrx = trx;

    // Apply changes to upstream entities
    for (const entity of trx.entities) {
      if (entity.kind.type === 'Transacted') {
        const upstream = entity.kind.upstream;
        const state = entity.toState();
        upstream.applyState(state);
      }
    }
  }
}

// ── Transaction Tests ──

describe('Transaction', () => {
  test('constructor initializes with alive=true', () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);

    expect(trx.alive.value).toBe(true);
    expect(trx.entities).toHaveLength(0);
    expect(trx.createdEntityIds.size).toBe(0);
    expect(trx.id).toBeInstanceOf(TransactionId);
  });

  test('create() produces a mutable entity', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);

    const borrow = await trx.create(TestModel, { name: 'Alice', count: 42 });
    expect(borrow).toBeInstanceOf(MutableBorrow);
    expect(borrow.inner.id()).toBeDefined();
    expect(trx.entities).toHaveLength(1);
    expect(trx.createdEntityIds.size).toBe(1);
  });

  test('create() entity is writable', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);

    const borrow = await trx.create(TestModel, { name: 'Test' });
    const entity = borrow.inner.entity();
    expect(entity.isWritable()).toBe(true);
  });

  test('create() initializes fields from values', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);

    const borrow = await trx.create(TestModel, { name: 'Bob', count: 7 });
    const entity = borrow.inner.entity();

    const nameVal = entity.getPropertyValue('name');
    expect(nameVal).not.toBeNull();
    expect((nameVal as any).value).toBe('Bob');

    const countVal = entity.getPropertyValue('count');
    expect(countVal).not.toBeNull();
    expect((countVal as any).value).toBe(7);
  });

  test('create() tracks entity ID in createdEntityIds', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);

    const borrow = await trx.create(TestModel, { name: 'Alice' });
    const entityId = borrow.inner.id().toString();
    expect(trx.createdEntityIds.has(entityId)).toBe(true);
  });

  test('get() retrieves and forks existing entity', async () => {
    const ctx = new MockContext();

    // Pre-create an entity in the mock context
    const id = EntityId.new();
    const entity = Entity.create(id, 'test_trx' as any);
    const lww = entity.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Original' });
    ctx.entities.set(id.toString(), entity);

    const trx = new Transaction(ctx);
    const borrow = await trx.get(TestModel, id);
    expect(borrow).toBeInstanceOf(MutableBorrow);
    expect(trx.entities).toHaveLength(1);

    // Forked entity should be writable
    expect(borrow.inner.entity().isWritable()).toBe(true);
  });

  test('get() returns same fork on second call', async () => {
    const ctx = new MockContext();
    const id = EntityId.new();
    ctx.entities.set(id.toString(), Entity.create(id, 'test_trx' as any));

    const trx = new Transaction(ctx);
    const borrow1 = await trx.get(TestModel, id);
    const borrow2 = await trx.get(TestModel, id);

    // Same underlying entity (from transaction entity list)
    expect(borrow1.inner.entity()).toBe(borrow2.inner.entity());
  });

  test('edit() forks an entity into the transaction', () => {
    const ctx = new MockContext();
    const entity = Entity.create(EntityId.new(), 'test_trx' as any);
    const lww = entity.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Original' });

    const trx = new Transaction(ctx);
    const borrow = trx.edit(TestModel, entity);
    expect(borrow).toBeInstanceOf(MutableBorrow);
    expect(trx.entities).toHaveLength(1);
    expect(borrow.inner.entity().isWritable()).toBe(true);
  });

  test('edit() returns same fork on second call', () => {
    const ctx = new MockContext();
    const entity = Entity.create(EntityId.new(), 'test_trx' as any);

    const trx = new Transaction(ctx);
    const borrow1 = trx.edit(TestModel, entity);
    const borrow2 = trx.edit(TestModel, entity);
    expect(borrow1.inner.entity()).toBe(borrow2.inner.entity());
  });

  test('edit() isolates mutations from original', () => {
    const ctx = new MockContext();
    const entity = Entity.create(EntityId.new(), 'test_trx' as any);
    const lww = entity.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Original' });

    const trx = new Transaction(ctx);
    const borrow = trx.edit(TestModel, entity);

    // Mutate via fork
    const forkLww = borrow.inner.entity().getBackend(LWWBackend);
    forkLww.set('name', { type: 'String', value: 'Modified' });

    // Original unchanged
    expect((lww.get('name') as any).value).toBe('Original');
    // Fork has new value
    expect((forkLww.get('name') as any).value).toBe('Modified');
  });

  test('commit() delegates to context', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);
    await trx.create(TestModel, { name: 'Test' });
    await trx.commit();

    expect(ctx.commitCalled).toBe(true);
    expect(ctx.lastCommittedTrx).toBe(trx);
  });

  test('commit() marks alive as false', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);
    await trx.create(TestModel, { name: 'Test' });
    expect(trx.alive.value).toBe(true);

    await trx.commit();
    expect(trx.alive.value).toBe(false);
  });

  test('commit() makes forked entities non-writable', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);
    const borrow = await trx.create(TestModel, { name: 'Test' });
    const entity = borrow.inner.entity();
    expect(entity.isWritable()).toBe(true);

    await trx.commit();
    expect(entity.isWritable()).toBe(false);
  });

  test('rollback() marks alive as false', () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);
    expect(trx.alive.value).toBe(true);

    trx.rollback();
    expect(trx.alive.value).toBe(false);
  });

  test('rollback() makes all forked entities non-writable', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);
    const borrow1 = await trx.create(TestModel, { name: 'A' });
    const borrow2 = await trx.create(TestModel, { name: 'B' });

    expect(borrow1.inner.entity().isWritable()).toBe(true);
    expect(borrow2.inner.entity().isWritable()).toBe(true);

    trx.rollback();

    expect(borrow1.inner.entity().isWritable()).toBe(false);
    expect(borrow2.inner.entity().isWritable()).toBe(false);
  });

  test('double commit is rejected', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);
    await trx.create(TestModel, { name: 'Test' });
    await trx.commit();

    await expect(trx.commit()).rejects.toThrow('already committed');
  });

  test('commit after rollback is rejected', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);
    await trx.create(TestModel, { name: 'Test' });
    trx.rollback();

    await expect(trx.commit()).rejects.toThrow('already committed');
  });

  test('multiple entities in one transaction', async () => {
    const ctx = new MockContext();
    const trx = new Transaction(ctx);

    await trx.create(TestModel, { name: 'Alice' });
    await trx.create(TestModel, { name: 'Bob' });
    await trx.create(TestModel, { name: 'Charlie' });

    expect(trx.entities).toHaveLength(3);
    expect(trx.createdEntityIds.size).toBe(3);
  });
});

// ── Context Tests ──

describe('Context', () => {
  test('begin() creates a Transaction', () => {
    const mock = new MockContext();
    const ctx = new Context(mock);
    const trx = ctx.begin();

    expect(trx).toBeInstanceOf(Transaction);
    expect(trx.alive.value).toBe(true);
  });

  test('nodeId() delegates to inner', () => {
    const mock = new MockContext();
    const ctx = new Context(mock);
    const nodeId = ctx.nodeId();
    expect(nodeId).toBeDefined();
  });

  test('multiple transactions are independent', () => {
    const mock = new MockContext();
    const ctx = new Context(mock);

    const trx1 = ctx.begin();
    const trx2 = ctx.begin();

    expect(trx1).not.toBe(trx2);
    expect(trx1.id.equals(trx2.id)).toBe(false);
    expect(trx1.alive).not.toBe(trx2.alive);
  });
});

// ── EntityChange Tests ──

describe('EntityChange', () => {
  test('create() validates event entity IDs match', () => {
    const entity = Entity.create(EntityId.new(), 'test_trx' as any);
    const lww = entity.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Test' });

    // Create an event with a different entity ID
    const wrongId = EntityId.new();
    const ops = new OperationSet(new Map());
    const event = new Event('test_trx' as any, wrongId, ops, Clock.empty());
    const attested = new Attested(event);

    expect(() => EntityChange.create(entity, [attested])).toThrow();
  });

  test('create() validates event IDs are in head clock', () => {
    const entity = Entity.create(EntityId.new(), 'test_trx' as any);
    // Entity has empty head, but event has this entity's ID
    const ops = new OperationSet(new Map());
    const event = new Event('test_trx' as any, entity.id(), ops, Clock.empty());
    const attested = new Attested(event);

    // Event ID won't be in the entity's empty head
    expect(() => EntityChange.create(entity, [attested])).toThrow();
  });

  test('create() succeeds with valid event', () => {
    const entity = Entity.create(EntityId.new(), 'test_trx' as any);
    const lww = entity.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Test' });

    // Generate a real event
    const ops = new OperationSet(new Map());
    const event = new Event('test_trx' as any, entity.id(), ops, Clock.empty());

    // Put the event ID in the entity's head
    const eventId = event.id();
    entity.commitHead(Clock.fromEventId(eventId));

    const attested = new Attested(event);
    const change = EntityChange.create(entity, [attested]);
    expect(change.entity).toBe(entity);
    expect(change.events).toHaveLength(1);
  });

  test('intoParts() returns entity and events', () => {
    const entity = Entity.create(EntityId.new(), 'test_trx' as any);
    const ops = new OperationSet(new Map());
    const event = new Event('test_trx' as any, entity.id(), ops, Clock.empty());
    const eventId = event.id();
    entity.commitHead(Clock.fromEventId(eventId));

    const change = EntityChange.create(entity, [new Attested(event)]);
    const [e, evts] = change.intoParts();
    expect(e).toBe(entity);
    expect(evts).toHaveLength(1);
  });

  test('toString() includes collection and ID', () => {
    const entity = Entity.create(EntityId.new(), 'test_trx' as any);
    const ops = new OperationSet(new Map());
    const event = new Event('test_trx' as any, entity.id(), ops, Clock.empty());
    entity.commitHead(Clock.fromEventId(event.id()));

    const change = EntityChange.create(entity, [new Attested(event)]);
    expect(change.toString()).toContain('EntityChange');
    expect(change.toString()).toContain('test_trx');
  });
});

// ── ItemChange Tests ──

describe('ItemChange', () => {
  test('Initial change has item and empty events', () => {
    const change: ItemChange<string> = { kind: 'Initial', item: 'hello' };
    expect(itemChangeItem(change)).toBe('hello');
    expect(itemChangeEvents(change)).toHaveLength(0);
    expect(itemChangeKind(change)).toBe('Initial');
  });

  test('Add change has item and events', () => {
    const ops = new OperationSet(new Map());
    const event = new Event('test' as any, EntityId.new(), ops, Clock.empty());
    const attested = new Attested(event);

    const change: ItemChange<string> = { kind: 'Add', item: 'added', events: [attested] };
    expect(itemChangeItem(change)).toBe('added');
    expect(itemChangeEvents(change)).toHaveLength(1);
    expect(itemChangeKind(change)).toBe('Add');
  });

  test('Update change has item and events', () => {
    const change: ItemChange<number> = { kind: 'Update', item: 42, events: [] };
    expect(itemChangeItem(change)).toBe(42);
    expect(itemChangeKind(change)).toBe('Update');
  });

  test('Remove change has item and events', () => {
    const change: ItemChange<number> = { kind: 'Remove', item: 99, events: [] };
    expect(itemChangeItem(change)).toBe(99);
    expect(itemChangeKind(change)).toBe('Remove');
  });
});

// ── Event.id() Tests ──

describe('Event.id()', () => {
  test('produces a deterministic EventId', () => {
    const entityId = EntityId.new();
    const ops = new OperationSet(new Map());
    const parent = Clock.empty();

    const event1 = new Event('test' as any, entityId, ops, parent);
    const event2 = new Event('test' as any, entityId, ops, parent);

    expect(event1.id().equals(event2.id())).toBe(true);
  });

  test('different operations produce different IDs', () => {
    const entityId = EntityId.new();
    const parent = Clock.empty();

    const ops1 = new OperationSet(new Map());
    const ops2 = new OperationSet(new Map([['lww', [new Operation(new Uint8Array([1, 2, 3]))]]]));

    const event1 = new Event('test' as any, entityId, ops1, parent);
    const event2 = new Event('test' as any, entityId, ops2, parent);

    expect(event1.id().equals(event2.id())).toBe(false);
  });

  test('different entity IDs produce different IDs', () => {
    const ops = new OperationSet(new Map());
    const parent = Clock.empty();

    const event1 = new Event('test' as any, EntityId.new(), ops, parent);
    const event2 = new Event('test' as any, EntityId.new(), ops, parent);

    expect(event1.id().equals(event2.id())).toBe(false);
  });
});
