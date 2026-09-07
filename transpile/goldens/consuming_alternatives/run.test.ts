// Runs the emitted consuming_alternatives against the real runtime.
//
// Against the parent's engine `either` leaks two values — the `Shape` nobody
// marked moved and, on the `Two` path, the payload member the pattern did not
// name — while `nestedStruct` and `userOk` THROW: `v._0.is` and `o.unwrap` are
// methods neither value has.

import { expect, test } from 'bun:test';
import { Result, SeqCursor } from '@ankurah/base';
import { Duo, Outcome, Shape, Token, either, firstOk, nestedStruct, userOk } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an or-pattern takes the payload out of the alternative that matched', () => {
  expect(either(new Shape('One', { _0: new Token(4n) }))).toBe(4n);
});

test('and releases the member the pattern did not name', () => {
  expect(either(new Shape('Two', { _0: new Token(5n), _1: 9n }))).toBe(5n);
});

test('an alternative that matched nothing leaves the enum to its own arm', () => {
  expect(either(new Shape('Nothing', {}))).toBe(0n);
});

test('a nested struct is destructured rather than asked which variant it is', () => {
  expect(nestedStruct(new Duo('A', { _0: new Token(7n) }))).toBe(7n);
});

test("a crate's own Ok variant is opened as its own", () => {
  expect(userOk(new Outcome('Ok', { _0: new Token(3n) }))).toBe(3n);
});

test('and the other variant runs the else', () => {
  expect(userOk(new Outcome('Other', {}))).toBe(0n);
});

test('a genuine Result reached through a bound is still the wrapper', () => {
  expect(firstOk(new SeqCursor([Result.Ok<Token, Token>(new Token(11n))]))).toBe(11n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
