// MIRRORS: ankurah/tests/tests/nonexistent_entity.rs

import { describe, test, expect } from 'bun:test';
import { EntityId as EntityIdClass } from '@ankurah/proto';
import { MemoryStorageEngine } from '@ankurah/storage-memory';

import { Node } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { RetrievalError } from '../../src/error.ts';

// ── Model ──
// Mirrors: common.rs `struct Album { pub name: String, pub year: String }`
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// ── Helpers ──

function createDurableNode(): Node {
  const node = new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
  // Rust: node.system.create().await?;
  // Divergence: SystemManager not yet ported — skip [E8]
  return node;
}

// ── Tests ──

// Mirrors: get_nonexistent_entity_errors
describe('nonexistent_entity', () => {
  test('context.get() with a nonexistent entity ID returns an error', async () => {
    // Rust: let node = durable_sled_setup().await?;
    const node = createDurableNode();

    // Rust: let ctx = node.context(DEFAULT_CONTEXT)?;
    const ctx = node.context();

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
  test.skip('local node rejects phantom entity commits (requires conjure_evil_phantom)', () => {
    // Rust: let phantom = AlbumView::from_entity(node.conjure_evil_phantom(EntityId::new(), Album::collection()));
    // Rust: phantom.edit(&trx)?.name().replace("inside your mind")?;
    // Rust: assert!(trx.commit().await.is_err());
    // conjure_evil_phantom is not ported to TS Node.
  });

  // Mirrors: server_rejects_update_for_nonexistent
  // Requires LocalProcessConnection which is not yet ported.
  test.skip('server rejects update events for nonexistent entities (requires LocalProcessConnection)', () => {
    // Rust: let _conn = LocalProcessConnection::new(&server, &client).await?;
    // This test requires @ankurah/connector-local which is not yet ported.
  });

  // Mirrors: server_rejects_create_for_existing
  // Requires LocalProcessConnection which is not yet ported.
  test.skip('server rejects create events for entities that already exist (requires LocalProcessConnection)', () => {
    // Rust: let _conn = LocalProcessConnection::new(&server, &client).await?;
    // This test requires @ankurah/connector-local which is not yet ported.
  });
});
