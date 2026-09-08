// Runs the emitted a_cursor_through_a_field against the real runtime.
//
// Against the parent's engine `stored` throws: the class declared `walk: I`,
// the construction site handed over the array it had, and `pull()`'s own
// `this.walk.next()` is a method an array has not got.

import { expect, test } from 'bun:test';
import { Token, paired, stored } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a holder built around a walk hands out its first element', () => {
  const held = stored([new Token(1n), new Token(2n), new Token(3n)]);
  expect(held?.n).toBe(1n);
  held?.drop();
});

test('a walk beside a plain field is still a walk', () => {
  expect(paired([new Token(4n), new Token(5n)], 9n)).toBe(9n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
