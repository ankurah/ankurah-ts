// Runs the emitted partial_order against the real runtime. At the parent both
// `Ord::cmp` and a written-out `PartialOrd::partial_cmp` landed on `compareTo`,
// and whichever the source wrote first took the name — so `Weight(0)`, which
// Rust refuses to compare, compared like any other weight.

import { expect, test } from 'bun:test';
import { Plain, Weight } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the total order keeps `compareTo`, which is what a sort calls', () => {
  const a = new Weight(1);
  const b = new Weight(2);
  expect(a.compareTo(b)).toBe(-1);
  expect(b.compareTo(a)).toBe(1);
  expect(a.compareTo(a)).toBe(0);
  a.drop();
  b.drop();
});

test('and the partial order answers separately, including "not comparable"', () => {
  const zero = new Weight(0);
  const one = new Weight(1);
  expect(one.partialCompareTo(new Weight(2))).toBe(-1);
  // The defective path: Rust answers `None` here and the port answered `-1`,
  // because `Ord::cmp` was what ran.
  expect(zero.partialCompareTo(one)).toBe(null);
  expect(one.partialCompareTo(zero)).toBe(null);
  zero.drop();
  one.drop();
});

test('a forwarding partial order is the same method, and is not written twice', () => {
  const a = new Plain(1);
  const b = new Plain(2);
  expect(a.compareTo(b)).toBe(-1);
  expect((a as unknown as { partialCompareTo?: unknown }).partialCompareTo).toBe(undefined);
  a.drop();
  b.drop();
});

test('nothing leaked and nothing was dropped twice', () => {
  expectNoOwnershipReports();
});
