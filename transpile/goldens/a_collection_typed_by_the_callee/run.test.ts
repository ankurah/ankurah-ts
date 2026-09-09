// Runs the emitted a_collection_typed_by_the_callee against the runtime.
//
// A parameter written out in full types the argument handed to it, so `found`
// has an element and the `Tag`s it collected are released.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { run } from './input.ts';

test('a collection the callee fills is released by the caller that owns it', () => {
  expect(run()).toBe(3);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
