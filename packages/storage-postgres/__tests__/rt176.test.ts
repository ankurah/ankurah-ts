// MIRRORS: ankurah/storage/postgres/tests/rt176.rs
//
// RT176: get_state should return EntityNotFound for non-existent entities (postgres)
//
// When get_state is called for an entity that doesn't exist in postgres storage,
// it should throw RetrievalError with kind 'EntityNotFound' (not a generic StorageError).

import { describe, test } from 'bun:test';

// Integration test — requires:
// 1. Running Postgres (POSTGRES_URL env var)
// 2. Node + StorageCollection direct access
// 3. Bincode serialization stubs connected

describe.skip('rt176', () => {
  // Rust: fn postgres_get_state_returns_entity_not_found
  test('postgres_get_state_returns_entity_not_found', async () => {
    // Get a collection (creates tables)
    // Generate a random EntityId
    // Call getState — should throw RetrievalError with kind 'EntityNotFound'
    // Verify the error contains the requested ID
  });
});
