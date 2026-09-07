// Runs the emitted partial_range_bounds against the real runtime.
//
// Against the parent's engine this file does not even LOAD: `between` emits
// `const _t0 = ;`, which no JavaScript engine will parse. Written so that it
// did, `rangeContains` would then throw "declares no order" on a type whose
// `PartialOrd` does not forward to `Ord`, and `kept` would be a hole with a
// release of `it` beside it.

import { expect, test } from 'bun:test';
import { Token, Version, between, betweenNames, kept, unknown, version } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a range over a partial order answers from partialCompareTo', () => {
  const five = version(5);
  expect(between(five)).toBe(true);
  five.drop();
  const nine = version(9);
  expect(between(nine)).toBe(false);
  nine.drop();
});

test('and an UNORDERED value is in no range that performs a comparison', () => {
  const nothing = unknown();
  expect(between(nothing)).toBe(false);
  nothing.drop();
});

test('the bounds a range BUILDS are released with it', () => {
  // `between` builds two `Version`s per call and drops both; nothing here
  // holds either of them, so the only way they are accounted for is the
  // release the statement writes.
  const four = version(4);
  expect(between(four)).toBe(true);
  four.drop();
});

test('bounds that name places are left where they are', () => {
  const lo = version(1);
  const hi = version(9);
  const v = version(5);
  expect(betweenNames(lo, hi, v)).toBe(true);
  lo.drop();
  hi.drop();
  v.drop();
});

test('an owning adaptor on a named iterator keeps what it kept and drops the rest', () => {
  const survivors = kept([new Token(1), new Token(2), new Token(3)]);
  expect(survivors.map((t) => t.n)).toEqual([2, 3]);
  for (const token of survivors) token.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
