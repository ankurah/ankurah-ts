// Runs the emitted a_cloned_option_owns_its_copy against the runtime.
//
// The map keeps the value it holds, so releasing the copy releases nothing a
// second time.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { idOfOneSubscription } from './input.ts';

test('the copy the caller releases is not the value the map holds', () => {
  expect(idOfOneSubscription()).toBe(7n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
