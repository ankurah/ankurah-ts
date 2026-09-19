// Runs the emitted a_cloned_chain against the real runtime.
//
// Against the parent engine the adaptor form handed back the caller's own
// Items and the loop released each of them: `BUG: Item was dropped twice`.
// The form with no adaptor copied at both engines, which is what makes the
// receiver's shape the deciding thing.

import { expect, test } from 'bun:test';
import { Item, keptOver, keptWhole } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an adaptor before the copy still hands back the caller-s own values', () => {
  const items = [new Item(1n), new Item(2n), new Item(3n)];
  expect(keptOver(items)).toBe(5n);
  // The argument kept what it lent, so a second walk answers again.
  expect(keptOver(items)).toBe(5n);
  for (const item of items) item.drop();
});

test('the same collection with no adaptor between', () => {
  const items = [new Item(1n), new Item(2n)];
  expect(keptWhole(items)).toBe(3n);
  expect(keptWhole(items)).toBe(3n);
  for (const item of items) item.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
