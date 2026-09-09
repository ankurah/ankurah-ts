// Runs the emitted a_callable_reached_through_what_holds_it against the runtime.
//
// Each call reaches the callable itself, and each callee an expression produced
// is released exactly once.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { consumedOnce, fromAnExpression, throughAHolder } from './input.ts';

test('a call reaches the callable inside what holds it', () => {
  expect(throughAHolder()).toBe(7n);
  expect(fromAnExpression()).toBe(7n);
  expect(consumedOnce()).toBe(9n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
