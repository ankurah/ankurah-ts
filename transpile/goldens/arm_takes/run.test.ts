// Runs the emitted arm_takes against the real runtime. What is under test is who
// owns what after an arm's pattern has run: the elements a partial tuple pattern
// did not name are released where the match ends, a struct-variant arm's fields
// are paired with the members they NAME, and the two shapes the port has no
// lowering for throw holding nothing rather than leaking — and a partial tuple
// inside a payload MEMBER, which used to be one of those, releases the position
// it did not name (H12).

import { expect, test } from 'bun:test';
import {
  Holder,
  Maybe,
  Named,
  Outer,
  Token,
  both,
  member,
  namedWhole,
  nestedPair,
  nothing,
  outOfOrder,
  partial,
  threeDeep,
  userSome,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a partial tuple pattern releases the element it did not name', () => {
  expect(partial([Token.new(1), Token.new(2)] as any)).toBe(1);
});

test('a tuple match that names nothing releases the whole tuple', () => {
  expect(nothing([Token.new(1), Token.new(2)] as any)).toBe(0);
});

test('a tuple match that names every element releases each of them once', () => {
  expect(both([Token.new(1), Token.new(2)] as any)).toBe(3);
});

test('a struct-variant arm naming its fields out of order drops each once', () => {
  expect(outOfOrder(new Named('V', { copy: 7, held: Token.new(3) }) as any)).toBe(3);
});

test('a user variant named Some is refused rather than leaking its wrapper', () => {
  const inner = new Maybe('Some', { _0: Token.new(4) }) as any;
  expect(() => userSome(new Outer('W', { _0: inner }) as any)).toThrow(/cannot both take a name/);
});

test('a partial tuple inside a payload member releases the position it did not name', () => {
  const first = Token.new(5);
  const second = Token.new(6);
  // The defective answer: a hole. The arm names `a` and drops it itself; the
  // element it did not name is the arm's too, and is released by position.
  expect(member(new Holder('Pair', { _0: [first, second] }) as any)).toBe(5);
  expect(first.isDropped).toBe(true);
  expect(second.isDropped).toBe(true);
});

test('a nested tuple position no name took is released by path', () => {
  // `((a, _), c)` names `a` and `c`; `pair[0][1]` is nobody's, and the walk
  // used to stop at the top level and leave it to the collector.
  expect(nestedPair([[Token.new(1), Token.new(2)], Token.new(3)])).toBe(4);
});

test('three levels are all walked, deepest leaf first', () => {
  expect(threeDeep([[[Token.new(4), Token.new(5)], Token.new(6)], Token.new(7)])).toBe(11);
});

test('a nested position a name owns is released through that name', () => {
  expect(namedWhole([[Token.new(8), Token.new(9)], Token.new(10)])).toBe(18);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
