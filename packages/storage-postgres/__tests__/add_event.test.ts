// MIRRORS: ankurah/storage/postgres/tests/add_event.rs

import { describe, test } from 'bun:test';

// Integration test — requires:
// 1. Running Postgres (POSTGRES_URL env var)
// 2. Node + defineModel() integration
// 3. Bincode serialization stubs connected

describe.skip('add_event postgres', () => {
  // Rust: fn add_event_postgres
  test('add_event_postgres', async () => {
    // Create a Postgres-backed node
    // Create an Album, commit
    // Edit the album (insert text), commit
    // Edit the album again (insert text), commit
    // Dump entity events — should have 3 events
  });
});
