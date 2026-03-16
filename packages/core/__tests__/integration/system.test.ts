// MIRRORS: ankurah/tests/tests/system.rs
//
// Integration tests for SystemManager behavior across node lifecycles.
//
// NOTE: Most tests in the Rust source require LocalProcessConnection (inter-node
// communication) and SledStorageEngine persistence across Node reconstructions
// (re-opening the same storage engine). MemoryStorageEngine is ephemeral — its
// data is lost when the engine instance is dropped.
//
// Tests that are single-node and don't require persistence across Node reconstructions
// are ported directly. Tests requiring inter-node connectivity or cross-construction
// persistence are skipped pending connector and storage engine porting.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { CollectionSet } from '../../src/collectionset.ts';
import { WeakEntitySet } from '../../src/entity.ts';
import { Reactor } from '../../src/reactor/index.ts';
import { SystemManager, SYSTEM_COLLECTION_ID } from '../../src/system.ts';

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
  // Divergence: Skipped — requires LocalProcessConnection and cross-construction
  // storage persistence (SledStorageEngine) [E8].
  test.skip('test_system_persistence_across_reconstruction', async () => {
    // Requires LocalProcessConnection and persistent storage.
  });

  // Mirrors: system.rs test_system_root_change_behavior
  // Divergence: Skipped — requires LocalProcessConnection, list_collections(),
  // and cross-construction storage persistence [E8].
  test.skip('test_system_root_change_behavior', async () => {
    // Requires LocalProcessConnection, persistent storage, list_collections(), hard_reset().
  });

  // Mirrors: system.rs test_ephemeral_cached_root_supports_offline_queries_after_restart
  // Divergence: Skipped — requires LocalProcessConnection and cross-construction
  // storage persistence [E8].
  test.skip('test_ephemeral_cached_root_supports_offline_queries_after_restart', async () => {
    // Requires LocalProcessConnection and persistent storage across Node reconstructions.
  });

  // Mirrors: system.rs test_ephemeral_cached_fetch_supports_offline_after_restart
  // Divergence: Skipped — requires LocalProcessConnection and cross-construction
  // storage persistence [E8].
  test.skip('test_ephemeral_cached_fetch_supports_offline_after_restart', async () => {
    // Requires LocalProcessConnection, persistent storage, nocache() fetch semantics.
  });
});
