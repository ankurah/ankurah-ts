// TS-ONLY: In-memory storage engine for testing

import { describe, test, expect, beforeEach } from 'bun:test';
import { MemoryStorageEngine } from '../src/index.ts';
import {
  CollectionId,
  EntityId,
  EventId,
  Clock,
  State,
  StateBuffers,
  EntityState,
  Event,
  OperationSet,
  Attested,
  AttestationSet,
} from '@ankurah/proto';
import { RetrievalError, LWWBackend } from '@ankurah/core';
import { Selection, Predicate, ComparisonOperator, Expr, Literal, PathExpr, OrderByItem, OrderDirection } from '@ankurah/ankql';
import type { Value } from '@ankurah/core';

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/**
 * Build an Attested<EntityState> with optional state buffers.
 * If no stateBuffersMap is provided, uses empty state buffers / empty clock.
 */
function makeAttestedState(
  entityId: EntityId,
  collectionId: CollectionId,
  stateBuffersMap?: Map<string, Uint8Array>,
): Attested<EntityState> {
  const buffers = stateBuffersMap
    ? new StateBuffers(stateBuffersMap)
    : StateBuffers.default();
  const state = new State(buffers, Clock.default());
  const entityState = new EntityState(entityId, collectionId, state);
  return new Attested(entityState, AttestationSet.default());
}

/**
 * Build an Attested<Event> with empty operations and an optional parent clock.
 */
function makeAttestedEvent(
  collectionId: CollectionId,
  entityId: EntityId,
  parent?: Clock,
): Attested<Event> {
  const ops = new OperationSet();
  const parentClock = parent ?? Clock.default();
  const event = new Event(collectionId, entityId, ops, parentClock);
  return new Attested(event, AttestationSet.default());
}

/**
 * Use LWWBackend to create a serialized state buffer containing named Value properties.
 * Returns a Map with a single "lww" key mapping to the serialized buffer.
 */
function makeLwwStateBuffer(
  properties: Record<string, Value>,
): Map<string, Uint8Array> {
  const backend = new LWWBackend();
  for (const [name, value] of Object.entries(properties)) {
    backend.set(name, value);
  }
  // Commit values by calling toOperations (marks them committed)
  backend.toOperations();
  const buffer = backend.toStateBuffer();
  const map = new Map<string, Uint8Array>();
  map.set('lww', buffer);
  return map;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('MemoryStorageEngine', () => {
  let engine: MemoryStorageEngine;

  beforeEach(() => {
    engine = new MemoryStorageEngine();
  });

  // 1. Engine creates collections on demand
  test('creates collections on demand, same id returns same instance', async () => {
    const id1 = new CollectionId('test');
    const id2 = new CollectionId('test');
    const id3 = new CollectionId('other');

    const coll1 = await engine.collection(id1);
    const coll2 = await engine.collection(id2);
    const coll3 = await engine.collection(id3);

    // Same collection id returns the same instance
    expect(coll1).toBe(coll2);
    // Different collection id returns a different instance
    expect(coll1).not.toBe(coll3);
  });

  // 2. getState throws on missing entity
  test('getState throws RetrievalError on missing entity', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);
    const unknownId = EntityId.new();

    await expect(coll.getState(unknownId)).rejects.toThrow(RetrievalError);
  });

  // 3. setState + getState round-trip
  test('setState + getState round-trip', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);
    const entityId = EntityId.new();

    const attested = makeAttestedState(entityId, collId);
    await coll.setState(attested);

    const retrieved = await coll.getState(entityId);
    expect(retrieved.payload.entityId.equals(entityId)).toBe(true);
    // Same reference stored
    expect(retrieved).toBe(attested);
  });

  // 4. setState overwrites
  test('setState overwrites previous state', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);
    const entityId = EntityId.new();

    const state1 = makeAttestedState(entityId, collId);
    const state2 = makeAttestedState(entityId, collId);
    await coll.setState(state1);
    await coll.setState(state2);

    const retrieved = await coll.getState(entityId);
    expect(retrieved).toBe(state2);
    expect(retrieved).not.toBe(state1);
  });

  // 5. addEvent is idempotent
  test('addEvent is idempotent', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);
    const entityId = EntityId.new();

    const event = makeAttestedEvent(collId, entityId);
    const eventId = event.payload.id();

    await coll.addEvent(event);
    await coll.addEvent(event); // Add same event again

    const events = await coll.getEvents([eventId]);
    expect(events.length).toBe(1);
  });

  // 6. getEvents returns found events, skips missing
  test('getEvents returns found events and skips missing', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);

    // Create 3 events with different entity IDs so they get different EventIds
    const e1 = makeAttestedEvent(collId, EntityId.new());
    const e2 = makeAttestedEvent(collId, EntityId.new());
    const e3 = makeAttestedEvent(collId, EntityId.new());

    await coll.addEvent(e1);
    await coll.addEvent(e2);
    await coll.addEvent(e3);

    // Create a missing EventId (from a different entity)
    const missingEventId = makeAttestedEvent(collId, EntityId.new()).payload.id();

    const results = await coll.getEvents([
      e1.payload.id(),
      e3.payload.id(),
      missingEventId,
    ]);

    expect(results.length).toBe(2);
  });

  // 7. fetchStates with True predicate
  test('fetchStates with True predicate returns all states', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);

    await coll.setState(makeAttestedState(EntityId.new(), collId));
    await coll.setState(makeAttestedState(EntityId.new(), collId));
    await coll.setState(makeAttestedState(EntityId.new(), collId));

    const selection = new Selection(Predicate.True());
    const results = await coll.fetchStates(selection);
    expect(results.length).toBe(3);
  });

  // 8. fetchStates with equality predicate
  test('fetchStates with equality predicate filters correctly', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);

    const alice1 = makeAttestedState(
      EntityId.new(),
      collId,
      makeLwwStateBuffer({ name: { type: 'String', value: 'Alice' } }),
    );
    const bob = makeAttestedState(
      EntityId.new(),
      collId,
      makeLwwStateBuffer({ name: { type: 'String', value: 'Bob' } }),
    );
    const alice2 = makeAttestedState(
      EntityId.new(),
      collId,
      makeLwwStateBuffer({ name: { type: 'String', value: 'Alice' } }),
    );

    await coll.setState(alice1);
    await coll.setState(bob);
    await coll.setState(alice2);

    const selection = new Selection(
      Predicate.Comparison(
        Expr.Path(PathExpr.simple('name')),
        ComparisonOperator.Equal(),
        Expr.Literal(Literal.String('Alice')),
      ),
    );

    const results = await coll.fetchStates(selection);
    expect(results.length).toBe(2);
  });

  // 9. fetchStates with ORDER BY
  test('fetchStates with ORDER BY sorts correctly', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);

    const s30 = makeAttestedState(
      EntityId.new(),
      collId,
      makeLwwStateBuffer({ age: { type: 'I32', value: 30 } }),
    );
    const s10 = makeAttestedState(
      EntityId.new(),
      collId,
      makeLwwStateBuffer({ age: { type: 'I32', value: 10 } }),
    );
    const s20 = makeAttestedState(
      EntityId.new(),
      collId,
      makeLwwStateBuffer({ age: { type: 'I32', value: 20 } }),
    );

    await coll.setState(s30);
    await coll.setState(s10);
    await coll.setState(s20);

    // ASC order
    const selectionAsc = new Selection(
      Predicate.True(),
      [new OrderByItem(PathExpr.simple('age'), OrderDirection.Asc())],
    );
    const ascResults = await coll.fetchStates(selectionAsc);
    expect(ascResults.length).toBe(3);
    expect(ascResults[0]).toBe(s10);
    expect(ascResults[1]).toBe(s20);
    expect(ascResults[2]).toBe(s30);

    // DESC order
    const selectionDesc = new Selection(
      Predicate.True(),
      [new OrderByItem(PathExpr.simple('age'), OrderDirection.Desc())],
    );
    const descResults = await coll.fetchStates(selectionDesc);
    expect(descResults.length).toBe(3);
    expect(descResults[0]).toBe(s30);
    expect(descResults[1]).toBe(s20);
    expect(descResults[2]).toBe(s10);
  });

  // 10. fetchStates with LIMIT
  test('fetchStates with LIMIT returns limited results', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);

    for (let i = 0; i < 5; i++) {
      await coll.setState(makeAttestedState(EntityId.new(), collId));
    }

    const selection = new Selection(Predicate.True(), null, 2);
    const results = await coll.fetchStates(selection);
    expect(results.length).toBe(2);
  });

  // 11. fetchStates empty collection
  test('fetchStates on empty collection returns empty array', async () => {
    const collId = new CollectionId('test');
    const coll = await engine.collection(collId);

    const selection = new Selection(Predicate.True());
    const results = await coll.fetchStates(selection);
    expect(results.length).toBe(0);
  });
});
