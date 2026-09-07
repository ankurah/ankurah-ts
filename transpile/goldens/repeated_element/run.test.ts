// Runs the emitted repeated_element against the real runtime.
//
// Against the parent's engine `shared()` answers `[true, true]`: the repeated
// value got no element expectation, so the `.into()` converted to nothing, and
// the `fill` put the same value in both slots. Both halves are wrong and
// neither said so at run time.

import { expect, test } from 'bun:test';
import { Held, listed, numbers, shared, words } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a value with an identity, repeated, is refused', () => {
  expect(() => shared()).toThrow('the port has no translation for this');
});

test('the comma-list form converts each value and keeps them apart', () => {
  const held = listed();
  expect(held.map((h) => h.flag)).toEqual([true, false]);
  expect(held[0]).toBeInstanceOf(Held);
  expect(held[0] === held[1]).toBe(false);
  for (const one of held) one.drop();
});

test('a number has nothing to share, and fill IS the clone', () => {
  expect(numbers()).toEqual([7, 7, 7]);
});

test('and so has a string', () => {
  expect(words()).toEqual(['', '']);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
