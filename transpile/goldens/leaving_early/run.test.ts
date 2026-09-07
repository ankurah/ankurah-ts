// Runs the emitted leaving_early against the real runtime.
//
// Each test takes the path the statement leaves EARLY by, which is the path
// nothing released anything on. Against the parent's engine all three leak: the
// rejected promise leaves the `?` wrapper holding its token, the second `?`
// returns with the first one's payload in hand, and `count` answers the array's
// length while every element the walk passed stays where it was.

import { expect, test } from 'bun:test';
import { Token, awaited, counted, twoPayloads } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a rejected await leaves the ? wrapper released', async () => {
  const rejected = Promise.reject(new Error('no'));
  await expect(awaited(new Token(9), rejected)).rejects.toThrow('no');
});

test('and an await that resolves hands the token on', async () => {
  const answer = (await awaited(new Token(8), Promise.resolve(3))).unwrap();
  expect(answer[0]).toBe(3);
  answer[1].drop();
});

test('a second ? that leaves releases the first payload', () => {
  expect(twoPayloads(true, false)).toBe(null);
});

test('and where both succeed the callee releases both', () => {
  expect(twoPayloads(true, true)).toBe(4);
});

test('count drains the sequence and the adaptor survivors with it', () => {
  expect(counted([new Token(1), new Token(2), new Token(3)])).toBe(2);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
