// MIRRORS: ankurah/storage/postgres/tests/rt176.rs
//
// RT176: get_state should return EntityNotFound for non-existent entities (postgres)
//
// When get_state is called for an entity that doesn't exist in postgres storage,
// it should throw RetrievalError with kind 'EntityNotFound' (not a generic StorageError).

import { describe, test, expect, beforeAll, afterAll } from 'bun:test';
import { RetrievalError } from '@ankurah/core';
import { CollectionId, EntityId } from '@ankurah/proto';
import {
  createPostgresContainer,
  stopPostgresContainer,
  createPostgresNode,
  type PostgresTestContext,
} from './common.ts';

let pgCtx: PostgresTestContext;

beforeAll(async () => {
  pgCtx = await createPostgresContainer();
}, 60_000);

afterAll(async () => {
  await stopPostgresContainer(pgCtx);
}, 30_000);

describe('rt176', () => {
  // Rust: fn postgres_get_state_returns_entity_not_found
  test('postgres_get_state_returns_entity_not_found', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Get a collection (this creates the tables)
    const collection = await ctx.collection(CollectionId.from('album'));

    // Generate a random entity ID that definitely doesn't exist
    const nonExistentId = EntityId.new();

    // Call getState directly on the storage collection
    let caught: unknown;
    try {
      await collection.getState(nonExistentId);
    } catch (e) {
      caught = e;
    }

    // Should be EntityNotFound, NOT a generic StorageError
    expect(caught).toBeInstanceOf(RetrievalError);
    const err = caught as RetrievalError;
    expect(err.kind).toBe('EntityNotFound');
  });
});
