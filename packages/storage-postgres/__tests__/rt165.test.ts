// MIRRORS: ankurah/storage/postgres/tests/rt165.rs
//
// RT165: PostgreSQL storage should be idempotent when inserting duplicate events
//
// Duplicate event insertions (e.g., from network retries, peer sync) should not
// cause errors. EventIDs are content-addressed (SHA256 hash of entity_id +
// operations + parent), so duplicate insertions are safe and should be
// idempotent - returning false on subsequent attempts rather than erroring.

import { describe, test, expect, beforeAll, afterAll } from 'bun:test';
import { CollectionId } from '@ankurah/proto';
import {
  createPostgresContainer,
  stopPostgresContainer,
  createPostgresNode,
  Album,
  type PostgresTestContext,
} from './common.ts';

let pgCtx: PostgresTestContext;

beforeAll(async () => {
  pgCtx = await createPostgresContainer();
}, 60_000);

afterAll(async () => {
  await stopPostgresContainer(pgCtx);
}, 30_000);

describe('rt165', () => {
  // Rust: fn postgres_duplicate_event_idempotency
  test('postgres_duplicate_event_idempotency', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Create an Album, commit
    const trx = ctx.begin();
    const albumBorrow = await trx.create(Album, { name: 'Test Album', year: '2024' });
    const albumId = albumBorrow.inner.id();
    await trx.commit();

    // Get collection to access storage directly
    const collection = await ctx.collection(CollectionId.from('album'));

    // Get the first event that was created
    const events = await collection.dumpEntityEvents(albumId);
    expect(events.length).toBe(1);
    const event = events[0];

    // Try to add the same event again — should be idempotent
    const result1 = await collection.addEvent(event);
    expect(result1).toBe(false);

    // Try again — should still be idempotent
    const result2 = await collection.addEvent(event);
    expect(result2).toBe(false);

    // Verify we still only have one event
    const eventsAfter = await collection.dumpEntityEvents(albumId);
    expect(eventsAfter.length).toBe(1);
  });
});
