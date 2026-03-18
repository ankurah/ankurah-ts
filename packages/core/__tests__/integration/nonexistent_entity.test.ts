// MIRRORS: ankurah/tests/tests/nonexistent_entity.rs

import { describe, test, expect } from 'bun:test';
import {
  EntityId as EntityIdClass,
  Event,
  OperationSet,
  Clock,
  EventId,
  TransactionId,
  CollectionId,
  NodeRequestBody,
  NodeResponseBody,
  Attested,
} from '@ankurah/proto';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { LocalProcessConnection } from '@ankurah/connector-local';

import { Node } from '../../src/node.ts';
import { PermissiveAgent, DEFAULT_CONTEXT } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { RetrievalError } from '../../src/error.ts';

// ── Model ──
// Mirrors: common.rs `struct Album { pub name: String, pub year: String }`
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// ── Helpers ──

async function createDurableNode(): Promise<Node> {
  const node = new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
  await node.system.create();
  return node;
}

function createEphemeralNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: false,
  });
}

// ── Tests ──

// Mirrors: get_nonexistent_entity_errors
describe('nonexistent_entity', () => {
  test('context.get() with a nonexistent entity ID returns an error', async () => {
    // Rust: let node = durable_sled_setup().await?;
    const node = await createDurableNode();

    // Rust: let ctx = node.context(DEFAULT_CONTEXT)?;
    const ctx = await node.contextAsync();

    // Rust: let result = ctx.get::<AlbumView>(EntityId::new()).await;
    // Rust: assert!(matches!(result, Err(RetrievalError::EntityNotFound(_))));
    const randomId = EntityIdClass.new();
    try {
      await ctx.get(Album, randomId);
      // Should not reach here
      expect(true).toBe(false);
    } catch (err) {
      expect(err).toBeInstanceOf(RetrievalError);
      expect((err as RetrievalError).kind).toBe('EntityNotFound');
    }
  });

  // Mirrors: local_rejects_phantom_commit
  // Requires conjure_evil_phantom which is not available on TS Node.
  test.skip('local node rejects phantom entity commits (requires conjureEvilPhantom)', () => {
    // Rust: let phantom = AlbumView::from_entity(node.conjure_evil_phantom(EntityId::new(), Album::collection()));
    // Rust: phantom.edit(&trx)?.name().replace("inside your mind")?;
    // Rust: assert!(trx.commit().await.is_err());
    // conjureEvilPhantom is not ported to TS Node.
  });

  // Mirrors: server_rejects_update_for_nonexistent
  test('server rejects update events for nonexistent entities', async () => {
    const server = await createDurableNode();
    const client = createEphemeralNode();
    const conn = await LocalProcessConnection.new(server, client);
    await client.system.waitSystemReady();

    // Rust: let fake_update = proto::Event { ... parent: proto::Clock::new([proto::EventId::from_bytes([1u8; 32])]) }
    const fakeEventId = EventId.fromBytes(new Uint8Array(32).fill(1));
    const fakeUpdate = new Event(
      Album.collection(),
      EntityIdClass.new(),
      new OperationSet(new Map()),
      Clock.from([fakeEventId]),
    );

    // Rust: client.request(server.id, &DEFAULT_CONTEXT, NodeRequestBody::CommitTransaction { ... })
    const resp = await client.request(
      server.id,
      DEFAULT_CONTEXT,
      new NodeRequestBody('CommitTransaction', {
        id: TransactionId.new(),
        events: [new Attested(fakeUpdate)],
      }),
    );

    // Rust: assert!(matches!(resp, proto::NodeResponseBody::Error(_)));
    expect(resp.is('Error')).toBe(true);

    conn.destroy();
  });

  // Mirrors: server_rejects_create_for_existing
  test('server rejects create events for entities that already exist', async () => {
    const server = await createDurableNode();
    const client = createEphemeralNode();
    const conn = await LocalProcessConnection.new(server, client);
    await client.system.waitSystemReady();

    // Create an entity on the server first
    const serverCtx = await server.contextAsync();
    let existingId: EntityIdClass;
    {
      const trx = serverCtx.begin();
      const album = await trx.create(Album, { name: 'Existing', year: '2024' });
      existingId = album.inner.id();
      await trx.commit();
    }

    // Try to send a create event for the same entity (parent is empty = create event)
    const fakeCreate = new Event(
      Album.collection(),
      existingId!,
      new OperationSet(new Map()),
      Clock.empty(), // empty parent = create event
    );

    const resp = await client.request(
      server.id,
      DEFAULT_CONTEXT,
      new NodeRequestBody('CommitTransaction', {
        id: TransactionId.new(),
        events: [new Attested(fakeCreate)],
      }),
    );

    // Rust: assert!(matches!(resp, proto::NodeResponseBody::Error(_)));
    expect(resp.is('Error')).toBe(true);

    conn.destroy();
  });
});
