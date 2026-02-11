// MIRRORS: ankurah/core/src/entity.rs
// Tests for Entity, EntityKind, WeakEntitySet

import { describe, expect, test } from 'bun:test';
import {
  EntityId,
  Clock,
  State,
  StateBuffers,
  OperationSet,
  Operation,
  Event,
  BincodeWriter,
} from '@ankurah/proto';

import { Entity, WeakEntitySet } from '../src/entity.ts';
import { LWWBackend } from '../src/property/backend/lww.ts';
import { YjsBackend } from '../src/property/backend/yjs.ts';

// Helper: create a collection ID (plain string)
const testCollection = 'test_entity' as any;
const testCollection2 = 'other_collection' as any;

describe('Entity', () => {
  // ── Construction ──

  test('create() produces a primary entity with empty state', () => {
    const id = EntityId.new();
    const entity = Entity.create(id, testCollection);

    expect(entity.id()).toBe(id);
    expect(entity.collection()).toBe(testCollection);
    expect(entity.head().isEmpty()).toBe(true);
    expect(entity.kind.type).toBe('Primary');
    expect(entity.isWritable()).toBe(false);
  });

  test('fromState() hydrates backends from state buffers', () => {
    // Create an entity with LWW data and serialize its state
    const id = EntityId.new();
    const original = Entity.create(id, testCollection);
    const lww = original.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Alice' });
    lww.set('age', { type: 'I32', value: 30 });

    const state = original.toState();

    // Hydrate from state
    const hydrated = Entity.fromState(id, testCollection, state);
    expect(hydrated.id()).toBe(id);
    expect(hydrated.collection()).toBe(testCollection);

    // Check that values round-tripped
    const hydratedLww = hydrated.getBackend(LWWBackend);
    const nameValue = hydratedLww.get('name');
    expect(nameValue).not.toBeNull();
    expect(nameValue!.type).toBe('String');
    expect((nameValue as any).value).toBe('Alice');

    const ageValue = hydratedLww.get('age');
    expect(ageValue).not.toBeNull();
    expect(ageValue!.type).toBe('I32');
    expect((ageValue as any).value).toBe(30);
  });

  // ── State Serialization ──

  test('toState() serializes all backends', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const lww = entity.getBackend(LWWBackend);
    lww.set('x', { type: 'I32', value: 42 });

    const state = entity.toState();
    expect(state.stateBuffers.get('lww')).toBeDefined();
    expect(state.head.isEmpty()).toBe(true);
  });

  test('toEntityState() includes identity', () => {
    const id = EntityId.new();
    const entity = Entity.create(id, testCollection);
    const es = entity.toEntityState();
    expect(es.entityId).toBe(id);
    expect(es.collection).toBe(testCollection);
  });

  // ── Backend Access ──

  test('getBackend() lazily creates backends', () => {
    const entity = Entity.create(EntityId.new(), testCollection);

    // First call creates the backend
    const lww1 = entity.getBackend(LWWBackend);
    expect(lww1).toBeInstanceOf(LWWBackend);

    // Second call returns same instance
    const lww2 = entity.getBackend(LWWBackend);
    expect(lww2).toBe(lww1);
  });

  test('getBackendByName() creates and caches backends', () => {
    const entity = Entity.create(EntityId.new(), testCollection);

    const lww = entity.getBackendByName('lww');
    expect(lww).toBeInstanceOf(LWWBackend);

    const yjs = entity.getBackendByName('yjs');
    expect(yjs).toBeInstanceOf(YjsBackend);

    // Same instance on subsequent call
    expect(entity.getBackendByName('lww')).toBe(lww);
  });

  test('getPropertyValue() searches all backends', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const lww = entity.getBackend(LWWBackend);
    lww.set('color', { type: 'String', value: 'red' });

    const value = entity.getPropertyValue('color');
    expect(value).not.toBeNull();
    expect(value!.type).toBe('String');
    expect((value as any).value).toBe('red');
  });

  test('getPropertyValue("id") returns entity ID', () => {
    const id = EntityId.new();
    const entity = Entity.create(id, testCollection);

    const value = entity.getPropertyValue('id');
    expect(value).not.toBeNull();
    expect(value!.type).toBe('EntityId');
    expect((value as any).value.equals(id)).toBe(true);
  });

  test('getPropertyValue() returns null for missing field', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    expect(entity.getPropertyValue('nonexistent')).toBeNull();
  });

  // ── initializeProperty ──

  test('initializeProperty with LWW string', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    entity.initializeProperty('name', 'Bob', 'lww');

    const value = entity.getPropertyValue('name');
    expect(value).not.toBeNull();
    expect(value!.type).toBe('String');
    expect((value as any).value).toBe('Bob');
  });

  test('initializeProperty with LWW number', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    entity.initializeProperty('count', 42, 'lww');

    const value = entity.getPropertyValue('count');
    expect(value).not.toBeNull();
    expect(value!.type).toBe('I32');
    expect((value as any).value).toBe(42);
  });

  test('initializeProperty with Yjs text', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    entity.initializeProperty('title', 'Hello World', 'yjs');

    const yjs = entity.getBackend(YjsBackend);
    expect(yjs.getString('title')).toBe('Hello World');
  });

  // ── Snapshot / Transaction Forking ──

  test('snapshot() creates a transacted fork', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const lww = entity.getBackend(LWWBackend);
    lww.set('x', { type: 'I32', value: 1 });

    const trxAlive = { value: true };
    const fork = entity.snapshot(trxAlive);

    expect(fork.id()).toBe(entity.id());
    expect(fork.collection()).toBe(entity.collection());
    expect(fork.kind.type).toBe('Transacted');
    expect(fork.isWritable()).toBe(true);
  });

  test('snapshot() isolates backend mutations', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const lww = entity.getBackend(LWWBackend);
    lww.set('x', { type: 'I32', value: 1 });

    const trxAlive = { value: true };
    const fork = entity.snapshot(trxAlive);

    // Mutate fork
    const forkLww = fork.getBackend(LWWBackend);
    forkLww.set('x', { type: 'I32', value: 99 });

    // Original unchanged
    const origValue = lww.get('x');
    expect(origValue).not.toBeNull();
    expect((origValue as any).value).toBe(1);

    // Fork has new value
    const forkValue = forkLww.get('x');
    expect(forkValue).not.toBeNull();
    expect((forkValue as any).value).toBe(99);
  });

  test('snapshot() with Yjs backend isolates text mutations', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const yjs = entity.getBackend(YjsBackend);
    yjs.insert('text', 0, 'hello');

    const fork = entity.snapshot({ value: true });
    const forkYjs = fork.getBackend(YjsBackend);
    forkYjs.insert('text', 5, ' world');

    // Original unchanged
    expect(yjs.getString('text')).toBe('hello');
    // Fork has appended text
    expect(forkYjs.getString('text')).toBe('hello world');
  });

  // ── isWritable() ──

  test('isWritable() is false for primary entities', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    expect(entity.isWritable()).toBe(false);
  });

  test('isWritable() is true for live transacted entities', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const trxAlive = { value: true };
    const fork = entity.snapshot(trxAlive);
    expect(fork.isWritable()).toBe(true);
  });

  test('isWritable() becomes false when transaction dies', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const trxAlive = { value: true };
    const fork = entity.snapshot(trxAlive);
    expect(fork.isWritable()).toBe(true);

    // Simulate transaction rollback
    trxAlive.value = false;
    expect(fork.isWritable()).toBe(false);
  });

  // ── Event Generation ──

  test('generateCommitEvent() returns null when no mutations', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    entity.getBackend(LWWBackend); // Create backend but don't mutate
    expect(entity.generateCommitEvent()).toBeNull();
  });

  test('generateCommitEvent() returns event with operations', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const trxAlive = { value: true };
    const fork = entity.snapshot(trxAlive);
    const lww = fork.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Test' });

    const event = fork.generateCommitEvent();
    expect(event).not.toBeNull();
    expect(event!.entityId).toBe(entity.id());
    expect(event!.collection).toBe(testCollection);
    expect(event!.parent.isEmpty()).toBe(true); // New entity, empty head
  });

  test('generateCommitEvent() includes operations from all backends', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const fork = entity.snapshot({ value: true });

    // Mutate both backends
    const lww = fork.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Test' });

    const yjs = fork.getBackend(YjsBackend);
    yjs.insert('title', 0, 'Hello');

    const event = fork.generateCommitEvent();
    expect(event).not.toBeNull();

    // Both backends should have operations
    const lwwOps = event!.operations.get('lww');
    expect(lwwOps).toBeDefined();
    expect(lwwOps!.length).toBeGreaterThan(0);

    const yjsOps = event!.operations.get('yjs');
    expect(yjsOps).toBeDefined();
    expect(yjsOps!.length).toBeGreaterThan(0);
  });

  // ── commitHead ──

  test('commitHead() updates the entity head', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    expect(entity.head().isEmpty()).toBe(true);

    const fakeEventId = new Uint8Array(32);
    fakeEventId[0] = 0x42;
    const { EventId: EventIdClass } = require('@ankurah/proto');
    const eventId = EventIdClass.fromBytes(fakeEventId);
    const newHead = Clock.fromEventId(eventId);

    entity.commitHead(newHead);
    expect(entity.head().isEmpty()).toBe(false);
    expect(entity.head().length).toBe(1);
  });

  // ── State round-trip ──

  test('full state round-trip with LWW and Yjs backends', () => {
    const id = EntityId.new();
    const entity = Entity.create(id, testCollection);

    // Set up LWW data
    const lww = entity.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Alice' });
    lww.set('active', { type: 'Bool', value: true });

    // Set up Yjs data
    const yjs = entity.getBackend(YjsBackend);
    yjs.insert('bio', 0, 'Hello world');

    // Serialize
    const state = entity.toState();

    // Deserialize
    const restored = Entity.fromState(id, testCollection, state);

    // Verify LWW
    const rLww = restored.getBackend(LWWBackend);
    expect((rLww.get('name') as any).value).toBe('Alice');
    expect((rLww.get('active') as any).value).toBe(true);

    // Verify Yjs
    const rYjs = restored.getBackend(YjsBackend);
    expect(rYjs.getString('bio')).toBe('Hello world');
  });

  // ── Display ──

  test('toString() includes collection and ID', () => {
    const entity = Entity.create(EntityId.new(), testCollection);
    const str = entity.toString();
    expect(str).toContain('Entity(');
    expect(str).toContain(testCollection);
  });
});

describe('WeakEntitySet', () => {
  test('create() produces a new entity', () => {
    const set = new WeakEntitySet();
    const entity = set.create(testCollection);
    expect(entity.kind.type).toBe('Primary');
    expect(entity.collection()).toBe(testCollection);
  });

  test('get() finds created entity', () => {
    const set = new WeakEntitySet();
    const entity = set.create(testCollection);

    const found = set.get(entity.id());
    expect(found).not.toBeNull();
    expect(found!.id()).toBe(entity.id());
  });

  test('get() returns null for unknown ID', () => {
    const set = new WeakEntitySet();
    const unknownId = EntityId.new();
    expect(set.get(unknownId)).toBeNull();
  });

  test('register() deduplicates by ID', () => {
    const set = new WeakEntitySet();
    const entity = set.create(testCollection);

    // Create another entity with same concept but register explicitly
    const entity2 = Entity.create(entity.id(), testCollection);
    set.register(entity2);

    // Should still return the original
    const found = set.get(entity.id());
    expect(found).toBe(entity);
  });

  test('withState() creates new entity if not resident', () => {
    const set = new WeakEntitySet();
    const id = EntityId.new();

    // Create a state with some data
    const tempEntity = Entity.create(id, testCollection);
    const lww = tempEntity.getBackend(LWWBackend);
    lww.set('x', { type: 'I32', value: 5 });
    const state = tempEntity.toState();

    const [changed, entity] = set.withState(id, testCollection, state);
    expect(changed).toBeNull(); // Not previously on node
    expect(entity.id().equals(id)).toBe(true);

    // Value should be present
    const value = entity.getPropertyValue('x');
    expect(value).not.toBeNull();
    expect((value as any).value).toBe(5);
  });

  test('withState() applies state to existing resident', () => {
    const set = new WeakEntitySet();
    const entity = set.create(testCollection);
    const id = entity.id();

    // Create a state
    const tempEntity = Entity.create(id, testCollection);
    const lww = tempEntity.getBackend(LWWBackend);
    lww.set('x', { type: 'I32', value: 10 });
    const state = tempEntity.toState();

    const [changed, returned] = set.withState(id, testCollection, state);
    expect(changed).toBe(true);
    expect(returned).toBe(entity); // Same instance
  });
});
