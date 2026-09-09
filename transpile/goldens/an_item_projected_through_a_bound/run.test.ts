// Runs the emitted an_item_projected_through_a_bound against the runtime.
//
// A bound and an argument are never the same type; what they agree on is what
// the bound PROJECTS, so the holder's element is the item read out of the
// argument and what `take` hands back is released.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { takeOne, Tag } from './input.ts';

test('a holder built through a bound carries the item the bound projects', () => {
  expect(takeOne([new Tag('aa'), new Tag('b')])).toBe(3);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
