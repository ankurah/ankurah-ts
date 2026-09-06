// Runs the emitted refusal_owns against the real runtime. What is under test is
// what a refused statement still owns: a `?` operand standing to the left of the
// refusal is evaluated, its temporary holds what it took, and the sequence the
// refused call was walking was never taken at all. Every one of them is released
// however the statement is left — and each release asks the runtime first,
// because a temporary the prefix produced may already have been consumed.

import { expect, test } from 'bun:test';
import { Token, movedThenRefused, nested, onlyRefused } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a refusal beside an evaluated `?` operand leaves nothing behind', () => {
  expect(() => nested(Token.new(1), [Token.new(2), Token.new(3)])).toThrow(/FromIterator/);
});

test('a refusal with nothing before it still owns what it was walking', () => {
  expect(() => onlyRefused([Token.new(4), Token.new(5)])).toThrow(/FromIterator/);
});

test('a refusal standing before a call owns what that call would have taken', () => {
  expect(() => movedThenRefused(Token.new(6), [Token.new(7)])).toThrow(/FromIterator/);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
