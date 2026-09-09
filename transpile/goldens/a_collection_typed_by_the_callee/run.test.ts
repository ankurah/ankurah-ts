// Runs the emitted a_collection_typed_by_the_callee against the runtime.
//
// Against 8ab991f's engine a constraint ran only where the DECLARED parameter
// carried an unknown, so `found` — handed to a parameter written out in full —
// stayed untyped and nothing released the `Tag`s it collected.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { run } from './input.ts';

test('a collection the callee fills is released by the caller that owns it', () => {
  expect(run()).toBe(3);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
