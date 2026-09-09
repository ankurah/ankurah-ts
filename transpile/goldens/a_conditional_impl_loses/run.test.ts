// Runs the emitted a_conditional_impl_loses against the runtime.
//
// The extension impl wants a bound the wrapper's element does not meet, so the
// call reaches through the `Deref` and answers 20 rather than 99.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { throughTheDeref } from './input.ts';

test('a conditional impl does not beat a method the deref chain reaches', () => {
  expect(throughTheDeref()).toBe(20);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
