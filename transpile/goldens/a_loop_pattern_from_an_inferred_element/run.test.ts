// Runs the emitted a_loop_pattern_from_an_inferred_element against the runtime.
//
// The two names a loop's pattern binds are the halves of the sequence's
// element, and each half is released when its turn ends.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { collected, describeBuilt } from './input.ts';

test('a loop over a sequence the body built releases both halves of each pair', () => {
  expect(describeBuilt()).toEqual(['created', '1']);
  expect(collected()).toBe(2);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
