// Runs the emitted a_cloned_sequence_owns_its_copies against the runtime.
//
// The map keeps the values it holds, so releasing the copies releases nothing
// a second time.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { totalOfEveryListener } from './input.ts';

test('the copies the caller releases are not the values the map holds', () => {
  expect(totalOfEveryListener()).toBe(3n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
