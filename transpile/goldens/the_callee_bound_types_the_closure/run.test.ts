// Runs the emitted the_callee_bound_types_the_closure against the real runtime.
//
// Every closure here is handed a `Token` BY VALUE, so the closure is what
// releases it. Against the parent's engine the parenthesised one and the one
// whose type came from a sibling actual released nothing, and the tokens were
// garbage collected undropped.

import { expect, test } from 'bun:test';
import { Token, fromASibling, parenthesised, plain } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a parenthesised closure releases what it was handed', () => {
  expect(parenthesised()).toBe(1n);
});

test('and so does the same closure without the parentheses', () => {
  expect(plain()).toBe(1n);
});

test('a closure typed from a sibling actual releases what it was handed', () => {
  expect(fromASibling(new Token(4n))).toBe(4n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
