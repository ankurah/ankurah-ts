// Runs the emitted range_shapes against the real runtime. Against the parent
// engine (b05f82c) `withinUnit` raises `TypeError: range(0, 1).contains is not
// a function`, `evensToTen` raises `.stepBy is not a function`, `letters`
// answers `["a"]` where Rust answers `['a', 'b', 'c']`, and `firstSlot` cannot
// tell "no element" from "an element that is None" — all four silently, with a
// diagnostic beside none of them.

import { expect, test } from 'bun:test';
import { evensToTen, firstPlain, firstSlot, letters, upTo16, within16, withinUnit } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a float range answers contains, from its bounds', () => {
  expect(withinUnit(0.5)).toBe(true);
  expect(withinUnit(0)).toBe(true);
  expect(withinUnit(1)).toBe(false);
  expect(withinUnit(-0.5)).toBe(false);
});

test('an integer range answers it the same way, half-open and closed', () => {
  expect(within16(0)).toBe(true);
  expect(within16(15)).toBe(true);
  expect(within16(16)).toBe(false);
  expect(upTo16(16)).toBe(true);
  expect(upTo16(17)).toBe(false);
});

test('step_by keeps every nth value', () => {
  expect(evensToTen()).toEqual([0, 2, 4, 6, 8]);
});

// A `char` range is the sequence of its code points, and the port has no helper
// for it. The parent answered `["a"]`: `'a' + 1` is the string `"a1"`, and
// `"a1" <= "c"` is false, so the loop stopped after one value.
test('a char range is refused rather than built out of string comparisons', () => {
  expect(() => letters()).toThrow(/code points/);
});

// E13: `Option<T>` is `T | null` here, so `Option<Option<T>>` has one `null`
// for two different answers.
test('a reader over a vector of Options is refused', () => {
  expect(() => firstSlot([null, 1])).toThrow(/cannot say whether there is no element/);
  // The same reader over a plain element is unchanged.
  expect(firstPlain([7, 8])).toBe(7);
  expect(firstPlain([])).toBe(null);
});

test('nothing leaked', async () => {
  await expectNoOwnershipReports();
});
