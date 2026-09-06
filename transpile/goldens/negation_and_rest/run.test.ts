// Runs the emitted negation_and_rest against the real runtime. Three answers
// the parent engine (c723a60) got wrong: `-i32::MIN` and `-i64::MIN` came back
// as values those widths cannot hold, and `Variant(..)` threw.

import { expect, test } from 'bun:test';
import { Wide, covered, firstOf, negate, negateFloat, negateWide, smallest } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('negating a signed integer raises where Rust raises', () => {
  expect(negate(5)).toBe(-5);
  expect(negate(-5)).toBe(5);
  // The defective answer: `2147483648`, which no `i32` holds.
  expect(() => negate(-2147483648)).toThrow('attempt to negate with overflow');
  expect(negate(2147483647)).toBe(-2147483647);
});

test('and so does a width the port holds in a bigint', () => {
  expect(negateWide(5n)).toBe(-5n);
  expect(() => negateWide(-9223372036854775808n)).toThrow('attempt to negate with overflow');
});

test('a float keeps the operator, and so does a literal', () => {
  expect(negateFloat(1.5)).toBe(-1.5);
  expect(negateFloat(0)).toBe(-0);
  // `-2147483648` is how `i32::MIN` is written; the helper would raise on it.
  expect(smallest()).toBe(-2147483648);
});

test('`Variant(..)` matches every value of that variant', () => {
  // The defective answer: the arm threw, because the pattern translator had no
  // test to write for `..`.
  const two = new Wide('Two', { _0: 7, _1: 8 });
  expect(covered(two)).toBe(2);
  two.drop();
  const one = new Wide('One', { _0: 4 });
  expect(covered(one)).toBe(4);
  one.drop();
  const nothing = new Wide('Nothing', {});
  expect(covered(nothing)).toBe(0);
  nothing.drop();
});

test('a trailing `..` covers the members the names before it did not take', () => {
  const two = new Wide('Two', { _0: 7, _1: 8 });
  expect(firstOf(two)).toBe(7);
  two.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
