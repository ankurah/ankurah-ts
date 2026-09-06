// Runs the emitted refusal_owns against the real runtime. What is under test is
// what a refused statement still owns: a `?` operand standing to the left of the
// refusal is evaluated, its temporary holds what it took, and the sequence the
// refused call was walking was never taken at all. Every one of them is released
// however the statement is left, each one under a flag this frame sets where
// the transfer is written.
//
// S1: that flag used to be the value's own `isMoved`/`isDropped`. A `Vec` is a
// plain array in the port and carries neither, so the guard always passed and a
// `Vec` an earlier `?` had already handed to a consuming call was dropped a
// second time. The last two tests are that case and its mirror; against the
// parent's engine the first of them reports `Token was dropped twice`.

import { expect, test } from 'bun:test';
import {
  Token,
  movedThenRefused,
  nested,
  onlyRefused,
  refusedInALoop,
  refusedInTheText,
  vecHandedOverFirst,
  vecNeverHandedOver,
} from './input.ts';
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

test('a Vec handed over by an earlier ? is not released again', () => {
  // `count(rest)` consumed the array before the refusal, so the `finally` must
  // leave it alone; `more` was never taken and must be released. Against the
  // parent's engine the guard read `undefined` off the array and dropped the
  // tokens `count` had already released.
  expect(() => vecHandedOverFirst([Token.new(8), Token.new(9)], [Token.new(10)])).toThrow(
    /FromIterator/,
  );
});

test('a Vec the refusal never reached is released once', () => {
  expect(() => vecNeverHandedOver([Token.new(11)], [Token.new(12), Token.new(13)])).toThrow(
    /FromIterator/,
  );
});

test('a refusal in the statement own text releases its parameters', () => {
  // `take2` is never entered, so both parameters are still this frame's.
  // Against the parent's engine neither is released and both are reported.
  expect(() => refusedInTheText(Token.new(14), [Token.new(15)])).toThrow(/FromIterator/);
});

test('a refusal inside a consuming loop releases the current element', () => {
  // The loop hands out one element per turn and its tail release starts after
  // the current index, so nothing but this reaches the element in hand.
  expect(() => refusedInALoop([[Token.new(16)], [Token.new(17)]])).toThrow(/FromIterator/);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
