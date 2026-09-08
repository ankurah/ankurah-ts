// Runs the emitted a_suspension_before_the_read against the real runtime.
//
// The first function is called with a promise that REJECTS, which is the path
// the parent's engine leaks on: the `?` wrapper is created above the statement,
// the await suspends below it, and nothing released the `Ok` payload when the
// rejection left.

import { expect, test } from 'bun:test';
import { Token, suspendsFirst, suspendsLast } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a rejection while the wrapper is in hand releases what it holds', async () => {
  const rejected = Promise.reject(new Error('no'));
  await expect(suspendsFirst(new Token(1n), rejected)).rejects.toThrow('no');
});

test('and the path that resolves hands the payload on', async () => {
  const answer = await suspendsFirst(new Token(2n), Promise.resolve(5n));
  expect(answer.unwrap()).toBe(7n);
});

test('an await that finishes after the ? needs no guard', async () => {
  const answer = await suspendsLast(3n);
  expect(answer.unwrap()).toBe(3n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
