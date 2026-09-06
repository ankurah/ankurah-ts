// Runs the emitted hole_beside_a_question against the real runtime. What is
// under test is the null test a `?` writes: it belongs to the operand, and a
// refusal somewhere else in the operand's subtree must not take it away.
//
// The driver never reaches the hole — no input carries 99 — so every call here
// is a call Rust answers. Against the parent's engine the second test returns
// NaN where Rust returns None: `const v = _r0` bound `null` and `checkedAdd`
// added one to it.

import { expect, test } from 'bun:test';
import { pick } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the ? answers its operand where the operand answered', () => {
  expect(pick([1, 2, 5], [])).toBe(6);
});

test('the ? leaves with None where the operand answered None', () => {
  // The whole point: the operand is `find_map(..)`, which answered nothing, so
  // the `?` returns. Without the test this reads `null + 1`.
  expect(pick([1, 2, 3], [])).toBe(null);
});

test('an empty sequence answers None too', () => {
  expect(pick([], [])).toBe(null);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
