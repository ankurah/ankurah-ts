// Runs the emitted moved_below_a_refusal against the real runtime.
//
// Both functions throw — that is what a refusal does — and against the parent's
// engine both leak the value the frame still owned when the hole threw: the
// element the loop's turn had been handed, and the parameter the last statement
// would have moved.

import { expect, test } from 'bun:test';
import { Token, aParameter, inALoop } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a refusal releases the element the turn was handed', () => {
  expect(() => inALoop([new Token(1n), new Token(2n)])).toThrow();
});

test('a refusal releases the parameter a later statement would have moved', () => {
  expect(() => aParameter(new Token(3n))).toThrow();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
