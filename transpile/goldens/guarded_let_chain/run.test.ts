// Runs the emitted guarded_let_chain against the real runtime.
//
// Against the parent's engine every guard-false case answers `undefined`
// instead of 7 and leaks the payload the pattern had already taken — three
// values, one per function.

import { Result } from '@ankurah/base';
import { Token, allowed, fromAResult, fromAnOption } from './input.ts';
import { expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';

test('a false guard over a Result runs the else', () => {
  expect(fromAResult(Result.Ok<Token, bigint>(new Token(4n)), false)).toBe(7n);
});

test('and a true one hands the payload to the branch', () => {
  expect(fromAResult(Result.Ok<Token, bigint>(new Token(4n)), true)).toBe(4n);
});

test('a false guard over a nullable runs the else', () => {
  expect(fromAnOption(new Token(5n), false)).toBe(7n);
});

test('and a true one hands the payload to the branch', () => {
  expect(fromAnOption(new Token(5n), true)).toBe(5n);
});

test('a guard that reads the binding is made after it is bound', () => {
  expect(allowed(new Token(6n))).toBe(6n);
  expect(allowed(new Token(0n))).toBe(7n);
});

test('a pattern that does not match still runs the else', () => {
  expect(fromAnOption(null, true)).toBe(7n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
