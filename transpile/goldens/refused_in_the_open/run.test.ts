// Runs the emitted refused_in_the_open against the real runtime.
//
// Every one of these throws — that is what an R12 hole does — so what the
// driver checks is what the throw LEAVES BEHIND. Against the parent's engine
// the first three leak `held`: the emitter's own `(() => { .. })()`, the `=>`
// inside a string, and an unrelated closure argument each put an arrow before
// the first `unsupported(`, and the rule read that as "the hole is inside a
// callable the statement passed", which suppressed the whole cleanup.

import { expect, test } from 'bun:test';
import { Token, arrowInAString, besideAClosure, blockExpression, insideACallable } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

const throws = (body: () => unknown): void => {
  expect(body).toThrow('the port has no translation for this');
};

test('a hole inside the emitter own IIFE still stops the statement', () => {
  throws(() => blockExpression(new Token(1), [1, 2]));
});

test('an arrow inside a string is not a callable', () => {
  throws(() => arrowInAString(new Token(2), [1, 2]));
});

test('nor is an unrelated closure argument beside the hole', () => {
  throws(() => besideAClosure(new Token(3), [1, 2]));
});

test('and a hole INSIDE a passed callable leaves what the call already took', () => {
  // `take2` ran and consumed the token before `apply` invoked the closure that
  // threw, so nothing here owes a release for it.
  throws(() => insideACallable(new Token(4)));
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
