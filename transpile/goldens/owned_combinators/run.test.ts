// Runs the emitted owned_combinators against the real runtime. Every one of
// these threw `TypeError: ... is not a function` on the branch that called its
// closure, and leaked the closure's captures on the branch that did not.

import { expect, test } from 'bun:test';
import { Token, andThenCapture, filterCapture, filterOwned, isSomeAndOwned, mapCapture, mapOrCapture, mapOrElseCapture, nested, okOrElseCapture, namedClosure } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the receiver is evaluated before the arguments', () => {
  // Rust runs source(1), source(2), eager(); the hoists used to run
  // source(2), eager(), source(1).
  expect(nested()).toBe(1);
});

test('a captured closure is called through invoke, on the branch that calls it', () => {
  expect(mapCapture(4, Token.new(3))).toBe(7);
  expect(andThenCapture(4, Token.new(3))).toBe(7);
  expect(mapOrCapture(4, Token.new(3))).toBe(7);
  expect(mapOrElseCapture(4, Token.new(3), Token.new(9))).toBe(7);
  expect(okOrElseCapture(4, Token.new(3)).unwrap()).toBe(4);
  expect(filterOwned(4, Token.new(3))).toBe(4);
  expect(isSomeAndOwned(4, Token.new(3))).toBe(true);
});

test('and released on the branch that does not', () => {
  expect(mapCapture(null, Token.new(3))).toBe(null);
  expect(andThenCapture(null, Token.new(3))).toBe(null);
  expect(mapOrCapture(null, Token.new(3))).toBe(0);
  expect(mapOrElseCapture(null, Token.new(3), Token.new(9))).toBe(9);
  expect(okOrElseCapture(null, Token.new(3)).unwrapErr()).toBe(3);
  expect(filterOwned(null, Token.new(3))).toBe(null);
  expect(isSomeAndOwned(null, Token.new(3))).toBe(false);
});

test('a closure that captures nothing droppable is still called where it stands', () => {
  const token = Token.new(3);
  expect(filterCapture(4, token)).toBe(4);
  expect(filterCapture(1, token)).toBe(null);
  token.drop();
});

test('a closure bound to a NAME is invoked, and released on the branch that skips it', () => {
  // `(f)(v)` on an `OwnedClosure` is a TypeError; `invoke(f, v)` is the call.
  expect(namedClosure(4, Token.new(3))).toBe(7);
  // The branch that never calls it still owns it: the leak check at the end of
  // this file is the assertion.
  expect(namedClosure(null, Token.new(3))).toBe(null);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
