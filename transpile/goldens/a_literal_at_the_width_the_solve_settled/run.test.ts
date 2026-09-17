// Runs the emitted a_literal_at_the_width_the_solve_settled.
//
// A `u64` is a bigint, and JavaScript refuses to mix one with a number.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { settledByAReturn, settledLater } from './input.ts';

test('a literal a later call settles is written at that width', () => {
  expect(settledLater()).toBe(4n);
});

test('a literal the return type settles is written at that width', () => {
  expect(settledByAReturn()).toBe(5n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
