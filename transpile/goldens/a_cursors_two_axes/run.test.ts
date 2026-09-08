// Runs the emitted a_cursors_two_axes against the real runtime.
//
// The two axes are the elements a cursor walks and the rest it still holds, and
// against the parent's engine each is wrong in its own way. A `&mut self`
// method was written through `takeRest()`, which marks the cursor moved, so the
// frame's own `walk.drop()` below it was a use after move — and where the
// method short-circuits, everything after the match had left the cursor and was
// owned by nobody. A walk over BORROWED items owned them, so a cursor built for
// `tokens.iter()` released the caller's tokens and the caller's own drop was a
// double drop.

import { expect, test } from 'bun:test';
import {
  Token,
  anyThenCount,
  borrowedShapes,
  every,
  findThenNext,
  howManyLeft,
  ownedShapes,
  seen,
  third,
} from './input.ts';
import { SeqCursor } from '@ankurah/base';
import { expectNoOwnershipReports } from './leaks.ts';

const walk = (...ns: number[]): SeqCursor<Token> =>
  new SeqCursor(ns.map((n) => new Token(BigInt(n))));

test('any stops at the first match and the frame drops what it did not reach', () => {
  expect(seen(walk(1, 2, 3))).toBe(true);
  expect(seen(walk(9, 8))).toBe(false);
});

test('all stops at the first failure', () => {
  expect(every(walk(1, 2))).toBe(true);
  expect(every(walk(1, 0, 5))).toBe(false);
});

test('nth drops what it stepped over and hands back the one it landed on', () => {
  const held = third(walk(1, 2, 3, 4));
  expect(held?.n).toBe(3n);
  held?.drop();
  expect(third(walk(1))).toBe(null);
});

test('size_hint reads the walk and consumes nothing', () => {
  const cursor = walk(1, 2, 3);
  expect(howManyLeft(cursor)).toBe(3);
  expect(howManyLeft(cursor)).toBe(3);
  cursor.drop();
});

test('find and then next: the second call finds the cursor alive', () => {
  const [found, after] = findThenNext(walk(1, 2, 3));
  expect(found?.n).toBe(2n);
  expect(after?.n).toBe(3n);
  found?.drop();
  after?.drop();
});

test('any and then count: the first leaves the walk, the second consumes it', () => {
  expect(anyThenCount(walk(1, 2, 3))).toBe(2);
  expect(anyThenCount(walk(7, 8))).toBe(0);
});

test('a walk over the caller tokens releases none of them', () => {
  const held = [new Token(1n), new Token(2n), new Token(3n)];
  expect(borrowedShapes(held)).toBe(4);
  // Still the caller's, all three of them, after three walks over them.
  for (const token of held) token.drop();
});

test('a walk over its own tokens releases what it did not reach', () => {
  expect(ownedShapes([new Token(1n), new Token(2n)])).toBe(true);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
