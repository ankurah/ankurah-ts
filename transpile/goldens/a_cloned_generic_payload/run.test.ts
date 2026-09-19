// Runs the emitted a_cloned_generic_payload against the real runtime.
//
// Against the parent engine the generic accessor handed back the map's own
// Holder and the monomorphic caller released it: `BUG: Holder was dropped
// twice`, once through each of the two accessors.

import { expect, test } from 'bun:test';
import { HashMap } from '@ankurah/base';
import { Bag, Holder, takeAll, takeOne } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

function bagOf(...held: bigint[]): Bag<Holder> {
  const inner = new HashMap<bigint, Holder>();
  for (const n of held) inner.insert(n, new Holder(n));
  return new Bag(inner);
}

test('the value the accessor hands back is the caller-s to release', () => {
  const bag = bagOf(7n);
  expect(takeOne(bag, 7n)).toBe(7n);
  // The map still holds its own Holder, so a second call answers again.
  expect(takeOne(bag, 7n)).toBe(7n);
  bag.drop();
});

test('the sequence form hands back one copy per value', () => {
  const bag = bagOf(2n, 3n);
  expect(takeAll(bag)).toBe(5n);
  expect(takeAll(bag)).toBe(5n);
  bag.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
