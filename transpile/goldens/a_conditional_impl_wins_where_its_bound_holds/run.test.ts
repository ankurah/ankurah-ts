// Runs the emitted a_conditional_impl_wins_where_its_bound_holds: the
// wrapper's element is `R`, which is `Red`, so the extension impl applies and
// the call answers 99 rather than reaching through the `Deref`.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { throughTheBound } from './input.ts';

test('a conditional impl answers where its bound holds', () => {
  expect(throughTheBound()).toBe(99);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
