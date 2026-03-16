// MIRRORS: ankurah/tests/tests/property_backends.rs
// Integration test: mixed property backends (YrsString + LWW) on a single model

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { Node } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText, lww } from '../../src/define-model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';
import { LWWBackend } from '../../src/property/backend/lww.ts';

// ── Visibility enum ──
// Mirrors: property_backends.rs `enum Visibility { Public, Unlisted, Private }`
// Divergence: TS uses string literal union instead of derive(Property) enum [E1].
// LWW stores it as Value { type: 'String', value: 'Public' | 'Unlisted' | 'Private' }.

// ── Model ──
// Mirrors: property_backends.rs `struct Video`
// - title: YrsString (active_type)
// - description: YrsString (active_type), Option<String> projected
// - visibility: LWW (active_type), Visibility projected
// - attribution: LWW (active_type), Option<String> projected
const Video = defineModel('video', {
  title: yrsText(),
  description: yrsText(),
  visibility: lww<string>(),
  // views: PNCounter — deferred (dead code in Rust)
  attribution: lww<string | null>(),
});

// ── Test ──
// Mirrors: property_backends.rs property_backends()

describe('property_backends integration', () => {
  test('property_backends', async () => {
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

    // Create a Video entity
    // Rust: let cat_video = trx.create(&Video { title: "Cat video #2918".into(), ... }).await?;
    const trx = ctx.begin();
    const catVideoBorrow = await trx.create(Video, {
      title: 'Cat video #2918',
      description: 'Test',
      visibility: 'Public',
      attribution: null,
    });

    const id = catVideoBorrow.inner.id();

    // Rust: cat_video.visibility().set(&Visibility::Unlisted)?;
    // Access the LWW backend directly to set visibility
    const lwwBackend = catVideoBorrow.inner.entity().getBackend(LWWBackend);
    lwwBackend.set('visibility', { type: 'String', value: 'Unlisted' });

    // Rust: cat_video.title().insert(15, " (Very cute)")?;
    // Access the Yjs backend directly for text operations
    const yjsBackend = catVideoBorrow.inner.entity().getBackend(YjsBackend);
    yjsBackend.insert('title', 15, ' (Very cute)');

    // Rust: trx.commit().await?;
    await trx.commit();

    // Rust: let video = ctx.get::<VideoView>(id).await?;
    const video = await ctx.get(Video, id);

    // Rust: assert_eq!(video.visibility().unwrap(), Visibility::Unlisted);
    expect(video.visibility()).toBe('Unlisted');

    // Rust: assert_eq!(video.title().unwrap(), "Cat video #2918 (Very cute)");
    expect(video.title()).toBe('Cat video #2918 (Very cute)');
  });
});
