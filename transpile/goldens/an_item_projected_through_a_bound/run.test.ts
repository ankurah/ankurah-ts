// Runs the emitted an_item_projected_through_a_bound against the runtime.
//
// Against 9099480's engine the holder's element is an unknown no constraint
// binds — a bound and an argument are never the same type — so what `take`
// hands back has no type and is released by nobody.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { takeOne, Tag } from './input.ts';

test('a holder built through a bound carries the item the bound projects', () => {
  expect(takeOne([new Tag('aa'), new Tag('b')])).toBe(3);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
