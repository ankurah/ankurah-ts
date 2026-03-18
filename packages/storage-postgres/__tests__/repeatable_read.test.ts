// MIRRORS: ankurah/storage/postgres/tests/repeatable_read.rs

import { describe, test } from 'bun:test';

// Integration test — requires:
// 1. Running Postgres (POSTGRES_URL env var)
// 2. Node + defineModel() integration (Album model with YrsString)
// 3. Bincode serialization stubs connected

describe.skip('repeatable_read', () => {
  // Rust: fn pg_repeatable_read
  test('pg_repeatable_read', async () => {
    // Create Album "I love cats"
    // Open read-only view
    // Start two concurrent transactions:
    //   trx2: "cats" -> "tofu"
    //   trx3: "love" -> "devour"
    // Verify uncommitted changes don't affect read view
    // Commit trx2, verify view updates to "I love tofu"
    // Commit trx3, verify view updates to "I devour tofu"
  });

  // Rust: fn pg_events
  test('pg_events', async () => {
    // Same as pg_repeatable_read but tests event creation
    // Create Album "I love cats"
    // Open read-only view
    // Start two concurrent transactions with same edits
    // Verify event-based CRDT merging
  });
});
