// Runs the emitted operators against the real runtime. What is under test is the
// value each operator produces, because every one of these differs between the
// two languages: `==` on two objects is identity in JavaScript and a value
// comparison in Rust, `/` on integers leaves a fraction, `~` produces a signed
// 32-bit number whatever it was given, and a `bigint` beside a `number` throws
// rather than adding.

import { expect, test } from 'bun:test';
import { Boxed, Charge, Left, Parcel, Right, Tag, Weight, bigger, borrowedSum, chargeNegated, combined, different, eagerAnd, eagerOr, flipped, genericSum, halves, heavier, indexed, laterLocal, negated, same, shift64, shiftAssign32, shiftAssign8, shifted, shifts, toF32, toI64, toU64, complemented as complemented_ } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('== compares values, not references', () => {
  const a = new Tag(5);
  const b = new Tag(5);
  expect(same(a, b)).toBe(true);
  expect(different(a, b)).toBe(false);
  const c = new Tag(6);
  expect(same(a, c)).toBe(false);
  expect(different(a, c)).toBe(true);
  a.drop();
  b.drop();
  c.drop();
});

test('integer division truncates towards zero', () => {
  expect(halves(7)).toBe(3);
  expect(halves(8)).toBe(4);
});

test('! on an integer flips its bits and stays in range', () => {
  expect(flipped(0)).toBe(4294967295);
  expect(flipped(4294967295)).toBe(0);
});

test('! on a boolean is the negation', () => {
  expect(negated(true)).toBe(false);
});

test('64-bit arithmetic is bigint arithmetic throughout', () => {
  expect(shifted(0n)).toBe(9223372036854775808n);
  expect(typeof shifted(1n)).toBe('bigint');
});

test('comparison on ordinary numbers is the JavaScript operator', () => {
  expect(bigger(3, 2)).toBe(true);
  expect(bigger(2, 3)).toBe(false);
});

test('an overloaded operator releases both operands, so the caller does not', () => {
  // Both used to be released twice: `add` takes them by value and drops them,
  // and the caller's `finally` dropped them again.
  const total = combined(new Weight('a', 1n), new Weight('b', 2n));
  expect(total.grams).toBe(3n);
  total.drop();
});

test('what the operator answers is a value the block owns and releases', () => {
  expect(heavier(new Weight('a', 60n), new Weight('b', 60n))).toBe(true);
  expect(heavier(new Weight('a', 1n), new Weight('b', 2n))).toBe(false);
});

test('an operator impl written for references is found and called', () => {
  // Rust looks operator impls up on the exact operand types — never through a
  // reference, never through `Deref`. Peeling the references off before the
  // lookup missed this impl and left the JavaScript `+` between two objects,
  // which is `[object Object][object Object]`.
  const left = new Left(4n);
  const right = new Right(5n);
  expect(borrowedSum(left, right)).toBe(9n);
  // Both are borrowed, so both are still the caller's.
  left.drop();
  right.drop();
});

test('a heterogeneous operand named by a later local still moves the left one', () => {
  // `add` consumes the Parcel; the block used to release it as well.
  expect(laterLocal(new Parcel(7n))).toBe(9n);
});

test('a generic impl\'s Output is a type, so the result is released', () => {
  expect(genericSum(new Boxed(1n), new Boxed(2n))).toBe(2n);
});

test('Rust\'s & and | on booleans evaluate both operands', () => {
  // `&&` would not have called `note` at all once `false` had decided the
  // answer, and `||` would not have called it once `true` had.
  const seen: number[] = [];
  expect(eagerAnd(false, seen)).toBe(false);
  expect(seen.length).toBe(1);
  expect(eagerOr(true, seen)).toBe(true);
  expect(seen.length).toBe(2);
});

test('a float cast into a 64-bit integer saturates and answers 0 for NaN', () => {
  // The port truncated and kept the low bits, so `1e30` became an arbitrary
  // number; and `BigInt(NaN)` threw `RangeError` where Rust answers 0.
  expect(toU64(42.7)).toBe(42n);
  expect(toU64(NaN)).toBe(0n);
  expect(toU64(-5)).toBe(0n);
  expect(toU64(1e30)).toBe(18446744073709551615n);
  expect(toU64(Infinity)).toBe(18446744073709551615n);
  expect(toI64(-1.9)).toBe(-1n);
  expect(toI64(NaN)).toBe(0n);
  expect(toI64(-1e30)).toBe(-9223372036854775808n);
});

test('every f32 destination rounds to single precision', () => {
  expect(toF32(16777217n)).toBe(16777216);
});

test('a compound bit operation wraps to its own type', () => {
  // `value <<= 31` on a u32 answered -2147483648, and `value <<= 7` on a u8
  // answered 256.
  expect(shiftAssign32(1)).toBe(2147483648);
  expect(shiftAssign8(2)).toBe(0);
});

test('a bigint shift keeps the low bits of its type', () => {
  // `u64::MAX << 1` grew to 0x1fffffffffffffffe.
  expect(shift64(0xffffffffffffffffn)).toBe(0xfffffffffffffffen);
});

test('a bigint shift beside a literal is a bigint on both sides', () => {
  // Inside a tuple this threw `Cannot mix BigInt and other types`.
  expect(shifts(1, 1, 1n)).toEqual([2147483648, 16, 1099511627776n]);
});

test('the unary operators and indexing go through their impls', () => {
  // `-object` is NaN and `object[0]` is undefined; both used to be written
  // without a word.
  const negated = chargeNegated(new Charge(5));
  expect(negated.amount).toBe(-5);
  negated.drop();
  // `Neg::neg` takes self by value, so the caller must not release it again.
  const complemented = complemented_(new Charge(0));
  expect(complemented.amount).toBe(-1);
  complemented.drop();
  const held = new Charge(9);
  expect(indexed(held)).toBe(9);
  held.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
