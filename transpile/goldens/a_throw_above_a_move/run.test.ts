// Runs the emitted a_throw_above_a_move against the real runtime.
//
// Every function is called down its THROWING path, where a move BELOW the throw
// has not happened and the block still owes the release.

import { expect, test } from 'bun:test';
import {
  Token,
  aThrowingElementBeforeTheMove,
  theBindingOfTheThrowingStatement,
  throwsAboveAMove,
  throwsAboveOneMove,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a throw above a move releases both values the frame still held', () => {
  expect(() => throwsAboveAMove(new Token(1n), new Token(2n), null)).toThrow();
  // And the path that does not throw hands them on, released by nobody here.
  const held = throwsAboveAMove(new Token(3n), new Token(4n), 9n);
  for (const token of held) token.drop();
});

test('a throw above one move releases it', () => {
  expect(() => throwsAboveOneMove(new Token(5n), null)).toThrow();
  throwsAboveOneMove(new Token(6n), 9n).drop();
});

test('a throwing element before the move releases what the frame still held', () => {
  // The element that can throw is written FIRST, so its early return leaves the
  // frame holding both tokens, and both are released.
  const left = aThrowingElementBeforeTheMove(new Token(7n), new Token(8n), false);
  expect(left.isErr()).toBe(true);
  // The driver owns what the call handed back, and the golden run checks that
  // the only leak that can surface is the transpiler's.
  left.drop();
  const answer = aThrowingElementBeforeTheMove(new Token(9n), new Token(10n), true);
  expect(answer.unwrap()).toEqual([1n, 19n]);
});

test('a value the throwing statement itself binds is nobody s to release', () => {
  expect(theBindingOfTheThrowingStatement(4n)).toBe(4n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
