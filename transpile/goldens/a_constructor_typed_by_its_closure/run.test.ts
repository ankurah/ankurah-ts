// Runs the emitted a_constructor_typed_by_its_closure against the runtime.
//
// A constructor written without its type argument takes one from the closure
// handed to it, so `counted.get()` resolves and what the constructor built is
// released.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { builtAndKept, width } from './input.ts';

test('a constructor takes its type from what the closure answers', () => {
  expect(width('four')).toBe(4);
});

test('a value the body keeps to itself is released where it was built', () => {
  expect(builtAndKept('four')).toBe(true);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
