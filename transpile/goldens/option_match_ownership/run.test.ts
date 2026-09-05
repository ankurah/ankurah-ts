// Runs the emitted option_match_ownership against the real runtime. What is
// under test is WHERE the arm's binding is declared: a `finally` is a sibling
// of its `try`, so a `const` declared inside the block is not a name the
// release can see. Written that way, every arm that owned what it bound threw
// `ReferenceError: token is not defined` on the way out and then leaked the
// value it was trying to release — live in storage-common's planner, whose
// release read `if (!_moved2) bounds.drop()` against a `const bounds` declared
// one block deeper.

import { expect, test } from 'bun:test';
import { Token, either, handOn, peek, read } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an arm that keeps what it bound releases it, and the release can see it', () => {
  expect(read(Token.new(4))).toBe(5);
});

test('the null side of the same match answers without touching a value', () => {
  expect(read(null)).toBe(0);
});

test('an arm that hands its binding on leaves nothing to release', () => {
  expect(handOn(Token.new(7))).toBe(7);
  expect(handOn(null)).toBe(0);
});

test('a match with one arm keeping and one handing on releases exactly once', () => {
  expect(either(Token.new(3), true)).toBe(103);
  expect(either(Token.new(3), false)).toBe(3);
  expect(either(null, true)).toBe(0);
});

test('a match through a reference leaves the value to its owner', () => {
  const token = Token.new(9);
  expect(peek(token)).toBe(9);
  expect(peek(null)).toBe(0);
  token.drop();
});

test('nothing leaked and nothing was dropped twice', () => {
  expectNoOwnershipReports();
});
