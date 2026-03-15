// MIRRORS: ankurah/core/src/system.rs

import { describe, expect, test } from 'bun:test';
import {
  EntityId,
  EventId,
  CollectionId,
  Clock,
  Event,
  Attested,
  EntityState,
  State,
  StateBuffers,
  Item,
} from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';

import { Entity, WeakEntitySet } from '../src/entity.ts';
import { CollectionSet } from '../src/collectionset.ts';
import { Reactor } from '../src/reactor/index.ts';
import type { StorageEngine, StorageCollection } from '../src/storage.ts';
import {
  SystemManager,
  SYSTEM_COLLECTION_ID,
  PROTECTED_COLLECTIONS,
  sysItemToValue,
  sysItemFromValue,
} from '../src/system.ts';
import { LWWBackend } from '../src/property/backend/lww.ts';
import { MutationError } from '../src/error.ts';

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

  async getEvents(eventIds: EventId[]): Promise<Attested<Event>[]> {
    return this.events.filter((e) =>
      eventIds.some((id) => id.equals(e.payload.id())),
    );
  }

  async fetchStates(_selection: Selection): Promise<Attested<EntityState>[]> {
    return [...this.states.values()];
  }
}

// ── Mock StorageEngine ──

class MockStorageEngine implements StorageEngine {
  readonly collections = new Map<string, MockStorageCollection>();

  async collection(id: CollectionId): Promise<StorageCollection> {
    const key = id.toString();
    let col = this.collections.get(key);
    if (!col) {
      col = new MockStorageCollection();
      this.collections.set(key, col);
    }
    return col;
  }
}

// ── Helper ──

function createSystemManager(opts?: { durable?: boolean }): {
  system: SystemManager;
  storage: MockStorageEngine;
  entities: WeakEntitySet;
  reactor: Reactor;
} {
  const storage = new MockStorageEngine();
  const collectionset = new CollectionSet(storage);
  const entities = new WeakEntitySet();
  const reactor = new Reactor();
  const durable = opts?.durable ?? false;

  const system = new SystemManager(collectionset, entities, reactor, durable);

  return { system, storage, entities, reactor };
}

// ── sysItemToValue / sysItemFromValue round-trip tests ──

describe('sysItemToValue / sysItemFromValue', () => {
  test('round-trip SysRoot', () => {
    const item = new Item('SysRoot', {});
    const value = sysItemToValue(item);
    expect(value).not.toBeNull();
    expect(value!.type).toBe('String');
    // Verify serde format: SysRoot -> "SysRoot" (JSON string)
    expect((value as { type: 'String'; value: string }).value).toBe('"SysRoot"');

    const parsed = sysItemFromValue(value);
    expect(parsed.type).toBe('SysRoot');
  });

  test('round-trip Collection', () => {
    const item = new Item('Collection', { name: 'my_collection' });
    const value = sysItemToValue(item);
    expect(value).not.toBeNull();
    expect(value!.type).toBe('String');
    // Verify serde format: Collection { name } -> {"Collection":{"name":"..."}}
    expect((value as { type: 'String'; value: string }).value).toBe(
      '{"Collection":{"name":"my_collection"}}',
    );

    const parsed = sysItemFromValue(value);
    expect(parsed.type).toBe('Collection');
    expect(parsed.is('Collection') && parsed.value.name).toBe('my_collection');
  });

  test('round-trip Other', () => {
    const item = new Item('Other', {});
    const value = sysItemToValue(item);
    expect(value).not.toBeNull();
    expect(value!.type).toBe('String');
    expect((value as { type: 'String'; value: string }).value).toBe('"Other"');

    const parsed = sysItemFromValue(value);
    expect(parsed.type).toBe('Other');
  });

  test('sysItemFromValue throws on null', () => {
    expect(() => sysItemFromValue(null)).toThrow();
  });

  test('sysItemFromValue throws on non-String value', () => {
    expect(() => sysItemFromValue({ type: 'I32', value: 42 })).toThrow();
  });

  test('sysItemFromValue throws on invalid JSON content', () => {
    expect(() =>
      sysItemFromValue({ type: 'String', value: '"InvalidVariant"' }),
    ).toThrow();
  });
});

// ── Constants tests ──

describe('constants', () => {
  test('SYSTEM_COLLECTION_ID is _ankurah_system', () => {
    expect(SYSTEM_COLLECTION_ID).toBe('_ankurah_system');
  });

  test('PROTECTED_COLLECTIONS contains SYSTEM_COLLECTION_ID', () => {
    expect(PROTECTED_COLLECTIONS).toContain(SYSTEM_COLLECTION_ID);
    expect(PROTECTED_COLLECTIONS).toHaveLength(1);
  });
});

// ── SystemManager construction and loadSystemCatalog ──

describe('SystemManager construction', () => {
  test('isLoaded becomes true after loading promise resolves', async () => {
    const { system } = createSystemManager();
    expect(system.isLoaded()).toBe(false);

    // Wait for loading to complete
    await system.waitLoaded();

    expect(system.isLoaded()).toBe(true);
  });

  test('isSystemReady is false initially for ephemeral node', async () => {
    const { system } = createSystemManager({ durable: false });
    await system.waitLoaded();
    expect(system.isSystemReady()).toBe(false);
  });

  test('root is null after loading empty storage', async () => {
    const { system } = createSystemManager();
    await system.waitLoaded();
    expect(system.root()).toBeNull();
  });

  test('getItems returns empty array after loading empty storage', async () => {
    const { system } = createSystemManager();
    await system.waitLoaded();
    expect(system.getItems()).toHaveLength(0);
  });
});

// ── SystemManager.create() ──

describe('SystemManager.create()', () => {
  test('creates system root on durable node', async () => {
    const { system, storage } = createSystemManager({ durable: true });
    await system.waitLoaded();

    await system.create();

    expect(system.isSystemReady()).toBe(true);
    expect(system.root()).not.toBeNull();
    expect(system.getItems()).toHaveLength(1);

    // Verify storage has the state and event
    const col = storage.collections.get(SYSTEM_COLLECTION_ID);
    expect(col).toBeDefined();
    expect(col!.states.size).toBe(1);
    expect(col!.events.length).toBe(1);
  });

  test('fails on non-durable node', async () => {
    const { system } = createSystemManager({ durable: false });
    await system.waitLoaded();

    await expect(system.create()).rejects.toThrow(
      'Only durable nodes can create a new system',
    );
  });

  test('fails if system root already exists', async () => {
    const { system } = createSystemManager({ durable: true });
    await system.waitLoaded();

    await system.create();

    await expect(system.create()).rejects.toThrow('System root already exists');
  });
});

// ── SystemManager.joinSystem() ──

describe('SystemManager.joinSystem()', () => {
  test('joins system on non-durable node', async () => {
    // First create a root state from a durable node
    const { system: durableSystem, storage: durableStorage } =
      createSystemManager({ durable: true });
    await durableSystem.waitLoaded();
    await durableSystem.create();

    const rootState = durableSystem.root()!;
    expect(rootState).not.toBeNull();

    // Now join from an ephemeral node
    const { system: ephemeralSystem } = createSystemManager({ durable: false });
    await ephemeralSystem.waitLoaded();

    await ephemeralSystem.joinSystem(rootState);

    expect(ephemeralSystem.isSystemReady()).toBe(true);
    expect(ephemeralSystem.root()).not.toBeNull();
  });

  test('fails on durable node', async () => {
    const { system: durableCreator } = createSystemManager({ durable: true });
    await durableCreator.waitLoaded();
    await durableCreator.create();
    const rootState = durableCreator.root()!;

    const { system: durableJoiner } = createSystemManager({ durable: true });
    await durableJoiner.waitLoaded();

    await expect(durableJoiner.joinSystem(rootState)).rejects.toThrow(
      'Durable nodes cannot join an existing system',
    );
  });

  test('matching root marks ready without re-storing', async () => {
    // Create a durable node and produce a root
    const { system: durableSystem } = createSystemManager({ durable: true });
    await durableSystem.waitLoaded();
    await durableSystem.create();
    const rootState = durableSystem.root()!;

    // First join to set up root
    const { system: ephemeralSystem, storage } = createSystemManager({
      durable: false,
    });
    await ephemeralSystem.waitLoaded();
    await ephemeralSystem.joinSystem(rootState);
    expect(ephemeralSystem.isSystemReady()).toBe(true);

    // Join again with the same root -- should succeed silently
    await ephemeralSystem.joinSystem(rootState);
    expect(ephemeralSystem.isSystemReady()).toBe(true);
  });
});

// ── SystemManager.hardReset() ──

describe('SystemManager.hardReset()', () => {
  test('clears all state', async () => {
    const { system, reactor } = createSystemManager({ durable: true });
    await system.waitLoaded();
    await system.create();

    expect(system.isSystemReady()).toBe(true);
    expect(system.root()).not.toBeNull();
    expect(system.getItems()).toHaveLength(1);

    await system.hardReset();

    expect(system.isSystemReady()).toBe(false);
    expect(system.root()).toBeNull();
    expect(system.getItems()).toHaveLength(0);
  });

  test('after reset, waitSystemReady blocks again', async () => {
    const { system } = createSystemManager({ durable: true });
    await system.waitLoaded();
    await system.create();

    expect(system.isSystemReady()).toBe(true);

    await system.hardReset();
    expect(system.isSystemReady()).toBe(false);

    // waitSystemReady should now block. Verify by racing with a timeout.
    let resolved = false;
    const waitPromise = system.waitSystemReady().then(() => {
      resolved = true;
    });

    // Give the microtask a chance to resolve (if it would)
    await new Promise((r) => setTimeout(r, 10));
    expect(resolved).toBe(false);
  });
});

// ── SystemManager.collection() ──

describe('SystemManager.collection()', () => {
  test('returns a StorageCollection after loading', async () => {
    const { system } = createSystemManager();
    // collection() internally calls waitLoaded()
    const col = await system.collection(
      CollectionId.fixedName(SYSTEM_COLLECTION_ID),
    );
    expect(col).toBeDefined();
  });
});

// ── loadSystemCatalog with pre-seeded storage ──

describe('SystemManager loadSystemCatalog with existing data', () => {
  test('durable node becomes ready when loading existing root', async () => {
    // Setup: seed the storage with a system root entity
    const storage = new MockStorageEngine();

    // Create an entity manually and store it
    const entityId = EntityId.new();
    const collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID);

    // Create LWW state buffer containing the "item" property with SysRoot value
    const tempEntity = Entity.create(entityId, collectionId);
    const lww = tempEntity.getBackend(LWWBackend);
    lww.set('item', sysItemToValue(new Item('SysRoot', {})));
    const entityState = tempEntity.toEntityState();
    const attestedState = new Attested(entityState);

    // Seed the storage
    const col = (await storage.collection(collectionId)) as MockStorageCollection;
    col.states.set(entityId.toString(), attestedState);

    // Now create a durable SystemManager with this pre-seeded storage
    const collectionset = new CollectionSet(storage);
    const entities = new WeakEntitySet();
    const reactor = new Reactor();
    const system = new SystemManager(collectionset, entities, reactor, true);

    await system.waitLoaded();

    // Durable node with existing root should be ready
    expect(system.isSystemReady()).toBe(true);
    expect(system.root()).not.toBeNull();
    expect(system.getItems()).toHaveLength(1);
  });

  test('ephemeral node becomes ready when loading existing cached root', async () => {
    // Same seed setup as above
    const storage = new MockStorageEngine();
    const entityId = EntityId.new();
    const collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID);

    const tempEntity = Entity.create(entityId, collectionId);
    const lww = tempEntity.getBackend(LWWBackend);
    lww.set('item', sysItemToValue(new Item('SysRoot', {})));
    const entityState = tempEntity.toEntityState();
    const attestedState = new Attested(entityState);

    const col = (await storage.collection(collectionId)) as MockStorageCollection;
    col.states.set(entityId.toString(), attestedState);

    // Create an ephemeral SystemManager
    const collectionset = new CollectionSet(storage);
    const entities = new WeakEntitySet();
    const reactor = new Reactor();
    const system = new SystemManager(collectionset, entities, reactor, false);

    await system.waitLoaded();

    // Ephemeral node with cached root becomes ready (enables offline-first)
    // Ephemeral nodes will verify/update the root when they connect via joinSystem()
    expect(system.isSystemReady()).toBe(true);
    expect(system.root()).not.toBeNull();
    expect(system.getItems()).toHaveLength(1);
  });
});
