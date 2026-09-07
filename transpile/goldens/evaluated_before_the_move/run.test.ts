// Runs the emitted evaluated_before_the_move against the real runtime. What is
// under test is who owns a value moved into a call when something the call has
// still to evaluate leaves the frame first.
//
// `laterThrows(token, null)` throws out of the `unwrap` that stands after the
// token in the argument list; `fieldAfterAQuestion(op, true)` leaves through
// the `?` in the field after the one that takes `op`. On both paths Rust drops
// the value it had not handed over, and against the parent's engine both were
// released by nobody and reported by the collector.

import { expect, test } from 'bun:test';
import { Op, Token, fieldAfterAQuestion, laterThrows } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the call hands back the sum when nothing throws', () => {
  expect(laterThrows(new Token(2), 3)).toBe(5);
});

test('the token is released when the argument after it throws', () => {
  const token = new Token(2);
  expect(() => laterThrows(token, null)).toThrow('unwrap');
  expect(token.isDropped).toBe(true);
});

test('the pair is built when the ? does not leave', () => {
  const pair = fieldAfterAQuestion(new Op(1), false);
  const built = pair.unwrap();
  expect(built.n).toBe(7);
  built.drop();
});

test('the field taken before the ? is released when the ? leaves', () => {
  const op = new Op(1);
  const failed = fieldAfterAQuestion(op, true);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
  expect(op.isDropped).toBe(true);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
