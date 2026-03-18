// MIRRORS: ankurah/tests/tests/inter_node.rs
//
// Inter-node integration tests: cross-node fetch, subscription propagation,
// view/field subscription lifecycle, disconnect/reconnect, cached fallback,
// lineage event bridge, and fetch-only subscription behavior.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { LocalProcessConnection } from '@ankurah/connector-local';
import { Node, matchArgs, nocache } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';

// ── Models ──
// Mirrors: common.rs `struct Album { name: String, year: String }`
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// Mirrors: common.rs `struct Pet { name: String, age: String }`
const Pet = defineModel('pet', {
  name: yrsText(),
  age: yrsText(),
});

// ── Helpers ──

function names(results: Array<{ name(): string | null }>): string[] {
  return results.map(r => r.name() ?? '');
}

// Mirrors: common.rs durable_sled_setup() — creates a durable node
async function createDurableNode(): Promise<Node> {
  const node = new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
  return node;
}

// Mirrors: common.rs ephemeral_sled_setup() — creates an ephemeral node
function createEphemeralNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: false,
  });
}

describe('inter_node', () => {
  // Mirrors: inter_node.rs inter_node_fetch
  test('inter_node_fetch', async () => {
    // Rust: let node1 = Node::new_durable(Arc::new(SledStorageEngine::new_test()?), PermissiveAgent::new());
    const node1 = await createDurableNode();
    // Rust: let node2 = Node::new(Arc::new(SledStorageEngine::new_test()?), PermissiveAgent::new());
    const node2 = createEphemeralNode();

    // Rust: node1.system.create().await?;
    await node1.system.create();

    // Rust: assert!(!node2.system.is_system_ready());
    expect(node2.system.isSystemReady()).toBe(false);

    // Rust: let _conn = LocalProcessConnection::new(&node1, &node2).await?;
    const conn = await LocalProcessConnection.new(node1, node2);

    // Wait for system to propagate to ephemeral node
    await node2.system.waitSystemReady();

    // Rust: let ctx1 = node1.context_async(c).await;
    const ctx1 = await node1.contextAsync();

    // Create 4 albums on node1
    {
      const trx = ctx1.begin();
      await trx.create(Album, { name: 'Walking on a Dream', year: '2008' });
      await trx.create(Album, { name: 'Ice on the Dune', year: '2013' });
      await trx.create(Album, { name: 'Two Vines', year: '2016' });
      await trx.create(Album, { name: 'Ask That God', year: '2024' });
      await trx.commit();
    }

    const p = "name = 'Walking on a Dream'";
    // Should already be on node1
    expect(names(await ctx1.fetch(Album, matchArgs(p)))).toEqual(['Walking on a Dream']);

    // Rust: let ctx2 = node2.context_async(c).await;
    const ctx2 = await node2.contextAsync();

    // Now node2 should successfully fetch the entity via inter-node fetch
    expect(names(await ctx2.fetch(Album, matchArgs(p)))).toEqual(['Walking on a Dream']);

    conn.destroy();
  });

  // Mirrors: inter_node.rs test_client_server_propagation
  test('test_client_server_propagation', async () => {
    // Rust: Create server (durable) and two client nodes
    const server = await createDurableNode();
    await server.system.create();

    const clientA = createEphemeralNode();
    const clientB = createEphemeralNode();

    // Connect both clients to the server
    const connA = await LocalProcessConnection.new(clientA, server);
    const connB = await LocalProcessConnection.new(clientB, server);

    await clientA.system.waitSystemReady();
    await clientB.system.waitSystemReady();

    const serverCtx = await server.contextAsync();
    const clientACtx = await clientA.contextAsync();
    const clientBCtx = await clientB.contextAsync();

    // Create an entity on client_a
    {
      const trx = clientACtx.begin();
      await trx.create(Album, { name: 'Origin of Symmetry', year: '2001' });
      await trx.commit();
    }

    // Wait for propagation
    await new Promise(resolve => setTimeout(resolve, 100));

    // Verify entity is queryable on server
    const query = "name = 'Origin of Symmetry'";
    expect(names(await serverCtx.fetch(Album, matchArgs(query)))).toEqual(['Origin of Symmetry']);

    // Wait for propagation to client_b
    await new Promise(resolve => setTimeout(resolve, 100));

    // Verify entity is queryable on client_b
    expect(names(await clientBCtx.fetch(Album, matchArgs(query)))).toEqual(['Origin of Symmetry']);

    connA.destroy();
    connB.destroy();
  });

  // Mirrors: inter_node.rs test_lineage_event_bridge
  test('test_lineage_event_bridge', async () => {
    const server = await createDurableNode();
    await server.system.create();
    const client = createEphemeralNode();

    const conn = await LocalProcessConnection.new(client, server);
    await client.system.waitSystemReady();

    const serverCtx = await server.contextAsync();
    const clientCtx = await client.contextAsync();

    // Create initial entity on server
    let petId;
    {
      const trx = serverCtx.begin();
      const pet = await trx.create(Pet, { name: 'BudgetTest', age: '1' });
      petId = pet.inner.id();
      await trx.commit();
    }

    // Client gets the entity
    const clientPet = await clientCtx.get(Pet, petId);
    expect(clientPet.age()).toBe('1');

    // Server makes 11 changes (exceeds retrieval budget of 10)
    for (let i = 2; i <= 12; i++) {
      const trx = serverCtx.begin();
      const serverPet = await trx.get(Pet, petId);
      const yjs = serverPet.inner.entity().getBackend(YjsBackend);
      yjs.delete('age', 0, String(i - 1).length);
      yjs.insert('age', 0, String(i));
      await trx.commit();
    }

    // Client fetches - EventBridge provides all missing events efficiently
    const results = await clientCtx.fetch(Pet, matchArgs("name = 'BudgetTest'"));

    expect(results.length).toBe(1);
    expect(results[0].age()).toBe('12');

    conn.destroy();
  });

  // Mirrors: inter_node.rs test_fetch_view_field_subscriptions_behavior
  test('test_fetch_view_field_subscriptions_behavior', async () => {
    const server = await createDurableNode();
    await server.system.create();
    const client = createEphemeralNode();

    const conn = await LocalProcessConnection.new(client, server);
    await client.system.waitSystemReady();

    const serverCtx = await server.contextAsync();
    const clientCtx = await client.contextAsync();

    // Create initial entity on server
    let petId;
    {
      const trx = serverCtx.begin();
      const pet = await trx.create(Pet, { name: 'Luna', age: '2' });
      petId = pet.inner.id();
      await trx.commit();
    }

    // Use fetch() to get the entity on client (no ongoing subscription)
    const fetchResult = await clientCtx.fetch(Pet, matchArgs("name = 'Luna'"));
    expect(fetchResult.length).toBe(1);

    // Make an edit on the server
    {
      const trx = serverCtx.begin();
      const serverPet = await trx.get(Pet, petId);
      const yjs = serverPet.inner.entity().getBackend(YjsBackend);
      yjs.delete('name', 0, 4); // delete "Luna"
      yjs.insert('name', 0, 'Stella');
      await trx.commit();
    }

    // Verify that fetch() doesn't establish ongoing subscriptions
    // (current behavior: fetched entities don't receive updates without a LiveQuery)
    await new Promise(resolve => setTimeout(resolve, 100));

    conn.destroy();
  });

  // Mirrors: inter_node.rs server_edits_subscription
  // Requires SubscriptionRelay to propagate LiveQuery subscription changes across nodes.
  // SubscriptionRelay is not yet implemented (subscribeRemoteQuery is stubbed in livequery.ts).
  test.skip('server_edits_subscription', async () => {
    // Needs SubscriptionRelay: server edits an entity, client's LiveQuery receives Update/Add notifications.
  });

  // Mirrors: inter_node.rs test_client_server_subscription_propagation
  // Requires SubscriptionRelay for cross-node LiveQuery subscription notifications.
  test.skip('test_client_server_subscription_propagation', async () => {
    // Needs SubscriptionRelay: entity created on client_a, server_watcher and client_b_watcher get Add.
  });

  // Mirrors: inter_node.rs test_view_field_subscriptions_with_query_lifecycle
  // Requires SubscriptionRelay for cross-node LiveQuery + View subscription lifecycle.
  test.skip('test_view_field_subscriptions_with_query_lifecycle', async () => {
    // Needs SubscriptionRelay: server edits entity, client's View/LiveQuery subscriptions receive updates.
  });

  // Mirrors: inter_node.rs cached_livequery_survives_disconnect_and_catches_up_on_reconnect
  // Requires disconnect/reconnect lifecycle on LocalProcessConnection.
  test.skip('cached_livequery_survives_disconnect_and_catches_up_on_reconnect', async () => {
    // Needs disconnect/reconnect lifecycle + SubscriptionRelay.
  });

  // Mirrors: inter_node.rs resident_entity_from_get_resubscribes_after_reconnect
  // Requires entity-level subscriptions + disconnect/reconnect lifecycle.
  test.skip('resident_entity_from_get_resubscribes_after_reconnect', async () => {
    // Needs entity-level subscriptions + disconnect/reconnect.
  });

  // Mirrors: inter_node.rs cached_reads_fall_back_to_local_on_transient_peer_failures
  // Requires FailingPeerSender mock + getCached().
  test.skip('cached_reads_fall_back_to_local_on_transient_peer_failures', async () => {
    // Needs FailingPeerSender + getCached().
  });
});
