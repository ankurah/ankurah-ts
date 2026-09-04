// TS-ONLY: bun:test hooks for suites that exercise the ownership runtime.
//
// A fatal latches the runtime, and in production it stays latched: a host that
// swallowed the throw cannot be trusted to keep running over corrupted
// ownership. A test suite is the one place that needs a per-test reset. Without
// it the first fatal fails every test after it, and the failure lands nowhere
// near the bug that caused it.
//
// The afterEach hook is the other half: if a fatal was raised during a test and
// something swallowed it, that test fails — so a swallowed fatal still fails the
// test that provoked it rather than leaking into the next one.
//
// The root bunfig.toml preloads this module for every test in the repo, and
// loading it installs the hooks — so a suite needs no setup of its own. Calling
// installOwnershipTestHooks() explicitly is still safe: it is idempotent, so a
// file that wants the dependency visible can call it without installing twice.
//
// Deliberately not re-exported from index.ts, because it imports bun:test and
// has no business in a production bundle.
//
// A suite that provokes a fatal on purpose asserts it and then calls
// clearFatalLatch(), which acknowledges it and keeps the afterEach quiet.

import { afterEach, beforeEach } from 'bun:test';
import { clearFatalLatch, isPoisoned } from './drop_registry.ts';

let installed = false;

export function installOwnershipTestHooks(): void {
  if (installed) return;
  installed = true;

  beforeEach(() => {
    clearFatalLatch();
  });

  afterEach(() => {
    if (!isPoisoned()) return;
    // Clear before failing, or this one unacknowledged fatal fails every test
    // that follows as well.
    clearFatalLatch();
    throw new Error(
      'BUG: a fatal ownership error was raised during this test and swallowed.\n' +
      'Something caught it and carried on, so the test ran over a runtime that\n' +
      'had already reported corrupted ownership. If the fatal is expected, assert\n' +
      'it and call clearFatalLatch() to acknowledge it.',
    );
  });
}

// Preloading is the supported wiring, so loading this module installs the hooks.
// Guarded because a host that loads it outside a test run has no hooks to
// register, and failing to install must not take the run down with it.
try {
  installOwnershipTestHooks();
} catch {
  // Not running under bun:test — nothing to install.
}
