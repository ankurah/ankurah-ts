// Runs the emitted a_loop_pattern_from_an_inferred_element against the runtime.
//
// The two names a loop's pattern binds are the halves of the sequence's
// element, and the sequence here is one the body built. Each half is owned by
// the turn that binds it and released when the turn ends; against 6f40781's
// engine the sequence has no element type, so the pair is never taken apart and
// nothing releases either half.

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
