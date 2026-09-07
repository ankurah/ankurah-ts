// Runs the emitted cursor_reborrow against the real runtime.
//
// The defective paths are both about a cursor the caller KEEPS. Against the
// parent's engine `sumOf` writes `walk.takeRest().takeRest()` — a method the
// array the first one answered has not got — and `drained` marks the cursor
// moved inside `drain` and then calls `walk.drop()` on it, which is a fatal use
// after move on a program Rust runs.

import { expect, test } from 'bun:test';
import { SeqCursor } from '@ankurah/base';
import { Token, allDrained, drain, drained, sumOf, summed } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a loop over by_ref drains the walk and leaves the cursor alive', () => {
  expect(summed([new Token(1n), new Token(2n), new Token(3n)])).toBe(6n);
});

test('a callee handed the reborrow drains it, and the owner still drops it', () => {
  expect(allDrained([new Token(4n), new Token(5n)])).toBe(9n);
});

test('the drained cursor is emptied, not moved, so its owner may still read it', () => {
  const walk = new SeqCursor([new Token(6n), new Token(7n)]);
  const rest = drain(walk);
  expect(rest.map((t) => t.n)).toEqual([6n, 7n]);
  for (const token of rest) token.drop();
  // Marked moved by the drain, every one of these would be fatal.
  expect(walk.remaining).toBe(0);
  walk.drop();
});

test('a cursor the loop never finishes releases what it did not reach', () => {
  // `sumOf` walks every element, so this is the whole sequence; what it proves
  // is that the elements are released once, by the loop, and not again by the
  // cursor.
  const walk = new SeqCursor([new Token(8n)]);
  expect(sumOf(walk)).toBe(8n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
