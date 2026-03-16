// MIRRORS: ankurah/tests/tests/rt106.rs
//
// Regression test for RT-106: Resubscription after unsubscribe should show
// up-to-date state, and missing events should be retrieved during lineage comparison.
//
// Divergence: Entirely skipped — all tests require LocalProcessConnection
// (client-server inter-node communication) which is not yet ported [E8].
// See port-runbook.md: @ankurah/connector-local — Not started.

import { describe, test } from 'bun:test';

describe('rt106', () => {
  // Mirrors: rt106.rs rt106
  // Requires: server + client nodes connected via LocalProcessConnection,
  // subscribe/unsubscribe/resubscribe lifecycle, dump_entity_events verification.
  test.skip('rt106', async () => {
    // Full test flow:
    // 1. Create album on server
    // 2. Client subscribes via LiveQuery (nocache) — sees the album
    // 3. Client unsubscribes (drops LiveQuery)
    // 4. Server makes 2 edits while client is unsubscribed
    // 5. Client resubscribes — should see year="2022" (latest state)
    // 6. Client should have retrieved 2 missing events during lineage comparison
    //
    // Requires LocalProcessConnection (not yet ported).
  });
});
