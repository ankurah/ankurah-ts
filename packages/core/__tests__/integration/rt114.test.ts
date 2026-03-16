// MIRRORS: ankurah/tests/tests/rt114.rs
//
// Regression test for RT-114: After resubscribing, entities that no longer match
// the predicate (due to server-side edits while unsubscribed) should not appear
// in the LiveQuery results.
//
// Divergence: Entirely skipped — all tests require LocalProcessConnection
// (client-server inter-node communication) which is not yet ported [E8].
// See port-runbook.md: @ankurah/connector-local — Not started.

import { describe, test } from 'bun:test';

describe('rt114', () => {
  // Mirrors: rt114.rs rt114
  // Requires: server + client nodes connected via LocalProcessConnection,
  // subscribe/unsubscribe/resubscribe lifecycle with predicate filtering.
  test.skip('rt114', async () => {
    // Full test flow:
    // 1. Create 2 albums on server (both year="2020")
    // 2. Client subscribes with "year >= '2020'" (nocache) — sees both albums
    // 3. Client unsubscribes (drops LiveQuery)
    // 4. Server changes album2's year to "2019" (no longer matches)
    // 5. Client resubscribes — should only see album1 (year="2020")
    //
    // Requires LocalProcessConnection (not yet ported).
  });

  // Mirrors: rt114.rs rt114_b
  // Same scenario as rt114 but using fetch() instead of LiveQuery.
  test.skip('rt114_b', async () => {
    // Full test flow:
    // 1. Create 2 albums on server (both year="2020")
    // 2. Client fetches with "year >= '2020'" (nocache) — sees both albums
    // 3. Server changes album2's year to "2019" (no longer matches)
    // 4. Client re-fetches — should only see album1 (year="2020")
    //
    // Requires LocalProcessConnection (not yet ported).
  });
});
