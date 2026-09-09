// Runs the emitted an_atomic_is_a_place_the_body_writes against the runtime.
//
// Every write lands, and `swap` and `compare_exchange` answer what the place
// held before them.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { claimedTwice, closed, counted, stored } from './input.ts';

test('an atomic is a place the body writes', () => {
  expect(counted()).toBe(2);
  expect(closed()).toBe(true);
  expect(claimedTwice()).toBe(true);
  expect(stored()).toBe(3);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
