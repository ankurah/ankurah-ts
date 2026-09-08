// Runs the emitted what_the_text_evaluates against the real runtime.
//
// Each function records the number of every call it makes, so the log IS the
// evaluation order. Against the parent's engine `inRange` logs 5, 1, 9 where
// rustc prints 1, 9, 5; `repeated` logs 2, 7 where rustc prints 7, 2; and
// `doubled` gives one value two names and two releases, which the ownership
// runtime reports as a double drop.

import { expect, test } from 'bun:test';
import { doubled, inRange, repeated } from './input.ts';
import { RefCell } from '@ankurah/base';
import { expectNoOwnershipReports } from './leaks.ts';

const log = (): RefCell<bigint[]> => new RefCell<bigint[]>([]);

/// What the cell holds, read through a borrow the reader gives back — a `Ref`
/// left outstanding is what `RefCell::drop` reports.
const seenBy = (cell: RefCell<bigint[]>): bigint[] => {
  const held = cell.borrow();
  const copy = [...held.value];
  held.drop();
  return copy;
};

test('a range contains evaluates start, end, then the item', () => {
  const seen = log();
  expect(inRange(seen)).toBe(true);
  expect(seenBy(seen)).toEqual([1n, 9n, 5n]);
  seen.drop();
});

test('a repeated vec evaluates the value, then the count', () => {
  const seen = log();
  expect(repeated(seen)).toEqual([7n, 7n]);
  expect(seenBy(seen)).toEqual([7n, 2n]);
  seen.drop();
});

test('a reference to a call is one value, named once and released once', () => {
  const seen = log();
  expect(doubled(seen)).toBe(3n);
  expect(seenBy(seen)).toEqual([3n]);
  seen.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
