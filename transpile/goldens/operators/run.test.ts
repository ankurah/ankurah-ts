// Runs the emitted operators against the real runtime. What is under test is the
// value each operator produces, because every one of these differs between the
// two languages: `==` on two objects is identity in JavaScript and a value
// comparison in Rust, `/` on integers leaves a fraction, `~` produces a signed
// 32-bit number whatever it was given, and a `bigint` beside a `number` throws
// rather than adding.

import { expect, test } from 'bun:test';
import { Tag, bigger, different, flipped, halves, negated, same, shifted } from './input.ts';
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

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
