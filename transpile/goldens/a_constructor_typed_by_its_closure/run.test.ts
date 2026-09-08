// Runs the emitted a_constructor_typed_by_its_closure against the runtime.
//
// Against 6f40781's engine `counted` has no type — `Calculated::new` does not
// resolve, because the path is written without its argument — so `counted.get()`
// is dispatched by name and nothing releases the value the constructor built.

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
