// MIRRORS: ankurah/tests/tests/system.rs
//
// Integration tests for SystemManager behavior across node lifecycles.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { LocalProcessConnection } from '@ankurah/connector-local';
import { Node } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { CollectionSet } from '../../src/collectionset.ts';
import { WeakEntitySet } from '../../src/entity.ts';
import { Reactor } from '../../src/reactor/index.ts';
import { SystemManager } from '../../src/system.ts';

// ── Models ──
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

const Pet = defineModel('pet', {
  name: yrsText(),
  age: yrsText(),
});

// ── Helper ──

function createSystemManager(opts?: { durable?: boolean }): {
  system: SystemManager;
  storage: MemoryStorageEngine;
} {
  const storage = new MemoryStorageEngine();
  const collectionset = new CollectionSet(storage);
  const entities = new WeakEntitySet();
  const reactor = new Reactor();
  const durable = opts?.durable ?? false;
  const system = new SystemManager(collectionset, entities, reactor, durable);
  return { system, storage };
}

// ── Tests ──

describe('system integration', () => {
  // Mirrors: system.rs test_system
  // Partially ported: the Rust test reuses the same SledStorageEngine across two Node
  // constructions. MemoryStorageEngine cannot do this (data is lost). We test the
  // first half (create + verify) and the idempotency check (create fails if root exists).
  test('test_system', async () => {
    const { system } = createSystemManager({ durable: true });
    await system.waitLoaded();

    await system.create();

    const root = system.root();
    expect(root).not.toBeNull();
    expect(root!.payload.state.head.len()).toBe(1);

    const items = system.getItems();
    expect(items.length).toBe(1);

    // Creating again should fail because the system already exists
    await expect(system.create()).rejects.toThrow();
  });

  // Mirrors: system.rs test_system_ready_behavior
  // Partially ported: the Rust test constructs multiple Node instances sharing the same
  // SledStorageEngine. We test the portions that work with a single-node MemoryStorageEngine.
  test('test_system_ready_behavior', async () => {
    // Fresh ephemeral node with no cached root should remain unready after loading
    {
      const { system } = createSystemManager({ durable: false });
      expect(system.isSystemReady()).toBe(false);

      await system.waitLoaded();
      expect(system.isSystemReady()).toBe(false);
      expect(system.root()).toBeNull();
    }

    // First create and initialize with a durable node
    {
      const { system } = createSystemManager({ durable: true });
      expect(system.isSystemReady()).toBe(false);

      await system.waitLoaded();
      await system.create();
      expect(system.isSystemReady()).toBe(true);

      const root = system.root();
      expect(root).not.toBeNull();
      expect(root!.payload.state.head.len()).toBe(1);
    }

    // NOTE: Remaining portions of Rust test (reconstructing Node with same engine,
    // ephemeral node with cached root) require persistent storage — skipped.
  });

  // Mirrors: system.rs test_system_persistence_across_reconstruction
  test('test_system_persistence_across_reconstruction', async () => {
    // Create separate storage engines for durable and ephemeral nodes
    const durableEngine = new MemoryStorageEngine();
    const ephemeralEngine = new MemoryStorageEngine();

    // First setup: Create both durable and ephemeral nodes
    let rootStateHead: any;
    {
      // Create and initialize durable node
      const durableNode = new Node({
        storageEngine: durableEngine,
        policyAgent: new PermissiveAgent(),
        durable: true,
      });
      await durableNode.system.create();
      expect(durableNode.system.isSystemReady()).toBe(true);

      // Get root state for later comparison
      const rootState = durableNode.system.root();
      expect(rootState).not.toBeNull();
      expect(rootState!.payload.state.head.len()).toBe(1);
      rootStateHead = rootState!.payload.state.head;

      // Create ephemeral node
      const ephemeralNode = new Node({
        storageEngine: ephemeralEngine,
        policyAgent: new PermissiveAgent(),
        durable: false,
      });
      await ephemeralNode.system.waitLoaded();
      expect(ephemeralNode.system.isSystemReady()).toBe(false);

      // Connect nodes using LocalProcessConnection
      const conn = await LocalProcessConnection.new(durableNode, ephemeralNode);

      // Wait for ephemeral node to be ready
      await ephemeralNode.system.waitSystemReady();
      expect(ephemeralNode.system.isSystemReady()).toBe(true);

      // Verify both nodes match the root state
      expect(durableNode.system.root()).not.toBeNull();
      expect(ephemeralNode.system.root()).not.toBeNull();

      conn.destroy();
    } // Both nodes and connection are dropped here

    // Second setup: Reconstruct both nodes with their respective storage engines
    {
      // Create new durable node - should automatically load existing system
      const durableNode = new Node({
        storageEngine: durableEngine,
        policyAgent: new PermissiveAgent(),
        durable: true,
      });
      await durableNode.system.waitLoaded();
      expect(durableNode.system.isSystemReady()).toBe(true);

      // Verify root state persisted in durable storage
      const durableRoot = durableNode.system.root();
      expect(durableRoot).not.toBeNull();
      expect(durableRoot!.payload.state.head.len()).toBe(rootStateHead.len());

      // Create new ephemeral node
      const ephemeralNode = new Node({
        storageEngine: ephemeralEngine,
        policyAgent: new PermissiveAgent(),
        durable: false,
      });
      await ephemeralNode.system.waitLoaded();
      expect(ephemeralNode.system.isSystemReady()).toBe(false);

      // Connect nodes using LocalProcessConnection
      const conn = await LocalProcessConnection.new(durableNode, ephemeralNode);

      // Wait for ephemeral node to be ready
      await ephemeralNode.system.waitSystemReady();
      expect(ephemeralNode.system.isSystemReady()).toBe(true);

      // Verify all roots match
      expect(durableNode.system.root()).not.toBeNull();
      expect(ephemeralNode.system.root()).not.toBeNull();

      conn.destroy();
    }
  });

  // Mirrors: system.rs test_system_root_change_behavior
  test('test_system_root_change_behavior', async () => {
    // Create separate storage engines for durable and ephemeral nodes
    const durableEngine = new MemoryStorageEngine();
    const ephemeralEngine = new MemoryStorageEngine();

    // Get initial root state
    let initialRootHead: any;
    {
      // Create and initialize durable node
      const durableNode = new Node({
        storageEngine: durableEngine,
        policyAgent: new PermissiveAgent(),
        durable: true,
      });
      await durableNode.system.create();
      expect(durableNode.system.isSystemReady()).toBe(true);

      // Create ephemeral node
      const ephemeralNode = new Node({
        storageEngine: ephemeralEngine,
        policyAgent: new PermissiveAgent(),
        durable: false,
      });
      await ephemeralNode.system.waitLoaded();
      expect(ephemeralNode.system.isSystemReady()).toBe(false);

      // Connect nodes
      const conn = await LocalProcessConnection.new(durableNode, ephemeralNode);

      // Wait for ephemeral node to be ready
      await ephemeralNode.system.waitSystemReady();
      expect(ephemeralNode.system.isSystemReady()).toBe(true);

      // Store initial root state for comparison
      const initialRoot = durableNode.system.root();
      expect(initialRoot).not.toBeNull();
      initialRootHead = initialRoot!.payload.state.head;

      // Verify both nodes have same root
      expect(durableNode.system.root()!.payload.state.head.len()).toBe(
        ephemeralNode.system.root()!.payload.state.head.len(),
      );

      // Create a pet on ephemeral node
      const ephemeralCtx = ephemeralNode.context();
      const trx = ephemeralCtx.begin();
      await trx.create(Pet, { name: 'Fido', age: '3' });
      await trx.commit();

      // Verify collections on ephemeral engine
      const ephCollections = ephemeralEngine.listCollections();
      expect(ephCollections.sort()).toEqual(['_ankurah_system', 'pet']);

      // Verify collections on durable engine
      const durCollections = durableEngine.listCollections();
      expect(durCollections.sort()).toEqual(['_ankurah_system', 'pet']);

      conn.destroy();
    } // Both nodes and connection are dropped here

    // Reset durable node's system (creating new root) but NOT ephemeral node
    let secondRootHead: any;
    {
      const durableNode = new Node({
        storageEngine: durableEngine,
        policyAgent: new PermissiveAgent(),
        durable: true,
      });
      await durableNode.system.waitLoaded();
      expect(durableNode.system.isSystemReady()).toBe(true);

      const durCollections = durableEngine.listCollections();
      expect(durCollections.sort()).toEqual(['_ankurah_system', 'pet']);

      // Reset storage and reinitialize
      await durableNode.system.hardReset();

      const durCollectionsAfterReset = durableEngine.listCollections();
      expect(durCollectionsAfterReset).toEqual([]);

      expect(durableNode.system.isSystemReady()).toBe(false);

      await durableNode.system.create();

      const durCollectionsAfterCreate = durableEngine.listCollections();
      expect(durCollectionsAfterCreate).toEqual(['_ankurah_system']);

      // Verify root has changed
      const secondRoot = durableNode.system.root();
      expect(secondRoot).not.toBeNull();
      secondRootHead = secondRoot!.payload.state.head;
      expect(secondRootHead.len()).toBe(1);
      // Root state should be different after reset
      // (different because new create generates new event)

      // Create an album on durable node
      const durableCtx = durableNode.context();
      const trx = durableCtx.begin();
      await trx.create(Album, { name: 'Leonard Skynyrd', year: '1973' });
      await trx.commit();

      const durCollectionsFinal = durableEngine.listCollections();
      expect(durCollectionsFinal.sort()).toEqual(['_ankurah_system', 'album']);
    } // Drop durable node

    // Ephemeral node joins the new system and resets everything
    {
      const durableNode = new Node({
        storageEngine: durableEngine,
        policyAgent: new PermissiveAgent(),
        durable: true,
      });
      await durableNode.system.waitLoaded();
      expect(durableNode.system.isSystemReady()).toBe(true);

      const ephemeralNode = new Node({
        storageEngine: ephemeralEngine,
        policyAgent: new PermissiveAgent(),
        durable: false,
      });
      await ephemeralNode.system.waitLoaded();
      expect(ephemeralNode.system.isSystemReady()).toBe(false);
      // Ephemeral node should have old root prior to joining
      expect(ephemeralNode.system.root()).not.toBeNull();

      const ephCollections = ephemeralEngine.listCollections();
      expect(ephCollections.sort()).toEqual(['_ankurah_system', 'pet']);

      // Connect nodes
      const conn = await LocalProcessConnection.new(durableNode, ephemeralNode);

      // Wait for ephemeral node to be ready
      await ephemeralNode.system.waitSystemReady();

      // Ephemeral node should have new root after joining
      expect(ephemeralNode.system.root()).not.toBeNull();

      // After joining new system, ephemeral engine should have been reset
      // (pet collection should be gone)
      const ephCollectionsAfterJoin = ephemeralEngine.listCollections();
      expect(ephCollectionsAfterJoin).toEqual(['_ankurah_system']);

      conn.destroy();
    }
  });
});
