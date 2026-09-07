// Runs the emitted shadowed_bindings against the real runtime.
//
// Every one of these leaks against the parent's engine, and each leak is one
// value: the outer name's, whose release was handed to the shadow's move.
// `shadowInAClosure` leaks twice there — the closure's parameter was typed by
// nothing, so nothing released what it held, and the shadow took the rest.

import { expect, test } from 'bun:test';
import { Token, shadowInAnArm, shadowInAClosure, shadowMoved, shadowReturned } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a shadow that is moved does not take the parameter release with it', () => {
  expect(shadowMoved(new Token(1n))).toBe(1n);
});

test('a shadow that is returned leaves the parameter owned', () => {
  const answer = shadowReturned(new Token(1n));
  expect(answer.n).toBe(9n);
  answer.drop();
});

test("a closure's parameter is typed by the callee's bound, and released", () => {
  expect(shadowInAClosure()).toBe(1n);
});

test("an arm's binding is still the arm's", () => {
  expect(shadowInAnArm()).toBe(2n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
