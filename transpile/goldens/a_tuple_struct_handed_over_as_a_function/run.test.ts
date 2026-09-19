// Runs the emitted a_tuple_struct_handed_over_as_a_function against the runtime.
// Against the parent engine both combinators wrote `(Wrapper)(x)`:
// `TypeError: Cannot call a class constructor Wrapper without |new|`.

import { expect, test } from 'bun:test';
import { Holder, Wrapper } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the constructor handed to a combinator constructs', () => {
  const holder = new Holder(7n, [2n, 3n]);
  const one = holder.wrapped();
  expect(one).toBeInstanceOf(Wrapper);
  expect(one!._0).toBe(7n);
  one!.drop();

  const all = holder.all();
  expect(all.map((w) => w._0)).toEqual([2n, 3n]);
  for (const wrapped of all) wrapped.drop();

  const spelled = holder.spelled();
  expect(spelled!._0).toBe(7n);
  spelled!.drop();
  holder.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
