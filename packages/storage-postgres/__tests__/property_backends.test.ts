// MIRRORS: ankurah/storage/postgres/tests/property_backends.rs

import { describe, test, expect, beforeAll, afterAll } from 'bun:test';
import { YjsBackend, LWWBackend } from '@ankurah/core';
import type { Value } from '@ankurah/core';
import {
  createPostgresContainer,
  stopPostgresContainer,
  createPostgresNode,
  Video,
  type PostgresTestContext,
} from './common.ts';

let pgCtx: PostgresTestContext;

beforeAll(async () => {
  pgCtx = await createPostgresContainer();
}, 60_000);

afterAll(async () => {
  await stopPostgresContainer(pgCtx);
}, 30_000);

describe('property_backends', () => {
  // Rust: fn pg_property_backends
  test('pg_property_backends', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    const trx = ctx.begin();
    const catVideo = await trx.create(Video, {
      title: 'Cat video #2918',
      description: 'Test',
      visibility: 'Public',
      attribution: null,
    });

    await trx.create(Video, {
      title: 'Cat video #9000',
      description: null,
      visibility: 'Unlisted',
      attribution: 'That guy',
    });

    const id = catVideo.inner.id();
    const entity = catVideo.inner.entity();

    // Modify visibility (LWW) — must pass Value type
    const lwwBackend = entity.getBackend(LWWBackend);
    lwwBackend.set('visibility', { type: 'String', value: 'Unlisted' } as Value);

    // Modify title (YrsString) — insert text
    const yjs = entity.getBackend(YjsBackend);
    yjs.insert('title', 15, ' (Very cute)');

    await trx.commit();

    // Verify via fetch
    const video = await ctx.get(Video, id);
    expect(video.visibility()).toBe('Unlisted');
    expect(video.title()).toBe('Cat video #2918 (Very cute)');
  });
});
