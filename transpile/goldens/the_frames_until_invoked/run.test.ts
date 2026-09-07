// Runs the emitted the_frames_until_invoked against the real runtime.
//
// Every case here throws or returns early on purpose, and against the parent's
// engine each one leaks exactly the values the frame had built or moved by then
// — one `Token` for the `?`, one for the tuple, one for the nested move, two
// for the pair of temporaries.

import { expect, test } from 'bun:test';
import { Token, afterAQuestion, inATuple, nestedInAnOperand, onlyTemporaries } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a `?` that leaves does not take the moved argument with it', () => {
  expect(afterAQuestion(new Token(1n), null)).toBe(null);
});

test('and one that does not leave still hands the argument over', () => {
  expect(afterAQuestion(new Token(1n), 2n)).toBe(3n);
});

test('a tuple element the throw never reached is still the frame\'s', () => {
  expect(() => inATuple(new Token(2n), null)).toThrow();
});

test('and a tuple that is built hands both elements over', () => {
  const pair = inATuple(new Token(2n), 5n);
  expect(pair[1]).toBe(5n);
  pair[0].drop();
});

test('a move nested inside a field is the frame\'s until the call is invoked', () => {
  expect(() => nestedInAnOperand(new Token(3n), new Token(4n), null)).toThrow();
});

test('and a construction that completes owns everything it was handed', () => {
  const built = nestedInAnOperand(new Token(3n), new Token(4n), 6n);
  expect(built.c).toBe(6n);
  built.drop();
});

test('temporaries built before a throw are the frame\'s, with no moved name', () => {
  expect(() => onlyTemporaries(null)).toThrow();
});

test('and are handed over when the construction completes', () => {
  const built = onlyTemporaries(7n);
  expect(built.c).toBe(7n);
  built.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
