// MIRRORS: ankurah/tests/tests/repeatable_read.rs
// Integration test: CRDT-based repeatable read isolation and merge

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { Node } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';

// ── Model ──
// Mirrors: repeatable_read.rs `struct Album { #[active_type(YrsString)] name: String }`
const Album = defineModel('album', {
  name: yrsText(),
});

// ── Test ──
// Mirrors: repeatable_read.rs repeatable_read()

describe('repeatable_read integration', () => {
  test('repeatable_read', async () => {
    // Rust: let node = Node::new_durable(Arc::new(SledStorageEngine::new_test().unwrap()), PermissiveAgent::new());
    const node = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: true,
    });

    // Rust: node.system.create().await?;
    // Divergence: SystemManager not yet ported — skip [E8]

    // Rust: let ctx = node.context(c)?;
    const ctx = node.context();

    // Create an Album with name = "I love cats"
    let id;
    {
      const trx = ctx.begin();
      const albumRw = await trx.create(Album, { name: 'I love cats' });

      // Rust: assert_eq!(album_rw.name().value(), Some("I love cats".to_string()));
      const yjsRw = albumRw.inner.entity().getBackend(YjsBackend);
      expect(yjsRw.getString('name')).toBe('I love cats');

      id = albumRw.inner.id();
      await trx.commit();
    }

    // Rust: let album_ro: AlbumView = ctx.get(id).await?;
    const albumRo = await ctx.get(Album, id);

    // Open two concurrent transactions editing the same entity
    // Rust: let trx2 = ctx.begin();
    // Rust: let album_rw2 = album_ro.edit(&trx2)?;
    const trx2 = ctx.begin();
    const albumRw2 = await trx2.get(Album, id);
    const yjs2 = albumRw2.inner.entity().getBackend(YjsBackend);

    // Rust: let trx3 = ctx.begin();
    // Rust: let album_rw3 = album_ro.edit(&trx3)?;
    const trx3 = ctx.begin();
    const albumRw3 = await trx3.get(Album, id);
    const yjs3 = albumRw3.inner.entity().getBackend(YjsBackend);

    // tx2: cats -> tofu
    // Rust: album_rw2.name().delete(7, 4)?;
    // Rust: album_rw2.name().insert(7, "tofu")?;
    yjs2.delete('name', 7, 4);
    yjs2.insert('name', 7, 'tofu');
    // Rust: assert_eq!(album_rw2.name().value(), Some("I love tofu".to_string()));
    expect(yjs2.getString('name')).toBe('I love tofu');

    // tx3: love -> devour
    // Rust: album_rw3.name().delete(2, 4)?;
    // Rust: album_rw3.name().insert(2, "devour")?;
    yjs3.delete('name', 2, 4);
    yjs3.insert('name', 2, 'devour');
    // Rust: assert_eq!(album_rw3.name().value(), Some("I devour cats".to_string()));
    expect(yjs3.getString('name')).toBe('I devour cats');

    // Both transactions are uncommitted — the read-only view should not be updated
    // Rust: assert_eq!(album_ro.name().unwrap(), "I love cats");
    const roVal1 = albumRo.entity().getPropertyValue('name');
    expect(roVal1).not.toBeNull();
    expect((roVal1 as any).value).toBe('I love cats');

    // Commit trx2
    // Rust: trx2.commit().await?;
    await trx2.commit();

    // Rust: assert_eq!(album_ro.name().unwrap(), "I love tofu");
    const roVal2 = albumRo.entity().getPropertyValue('name');
    expect(roVal2).not.toBeNull();
    expect((roVal2 as any).value).toBe('I love tofu');

    // Commit trx3
    // Rust: trx3.commit().await?;
    await trx3.commit();

    // Rust: assert_eq!(album_ro.name().unwrap(), "I devour tofu");
    // This is the CRDT merge: "I love cats" + (love→devour) + (cats→tofu) = "I devour tofu"
    const roVal3 = albumRo.entity().getPropertyValue('name');
    expect(roVal3).not.toBeNull();
    expect((roVal3 as any).value).toBe('I devour tofu');
  });
});
