// Runs the emitted a_bound_method_on_what_a_closure_answers against the runtime.
// Against the parent engine the call was untyped and `met` was written as a
// method: `TypeError: invokeRef(predicate, value).met is not a function`.

import { expect, test } from 'bun:test';
import { anyMet } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the boolean impl answers for a predicate that answers a bool', () => {
  expect(anyMet([1n, 2n, 3n], (value) => value === 2n)).toBe(true);
  expect(anyMet([1n, 3n], (value) => value === 2n)).toBe(false);
});

test('the option impl answers for a predicate that answers a nullable', () => {
  expect(anyMet([1n, 7n], (value) => (value > 5n ? value : null))).toBe(true);
  expect(anyMet([1n, 2n], (value) => (value > 5n ? value : null))).toBe(false);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
