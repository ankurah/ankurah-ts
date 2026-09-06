// Runs the emitted arm_takes against the real runtime. What is under test is who
// owns what after an arm's pattern has run: the elements a partial tuple pattern
// did not name are released where the match ends, a struct-variant arm's fields
// are paired with the members they NAME, and the two shapes the port has no
// lowering for throw holding nothing rather than leaking.

import { expect, test } from 'bun:test';
import { Holder, Maybe, Named, Outer, Token, both, member, nothing, outOfOrder, partial, userSome } from './input.ts';
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

test('a partial tuple inside a payload member is refused, holding nothing', () => {
  const pair = [Token.new(5), Token.new(6)] as any;
  expect(() => member(new Holder('Pair', { _0: pair }) as any)).toThrow(/only SOME of the elements/);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
