// Runs the emitted nested_exit against the real runtime. What is under test is
// where a `return` written inside a nested match ends up: it has to leave `run`
// and not the arm it was written in, and the arm still has to release the
// payload it took on the way past. ankql's `generate_expr_sql` is written this
// way, and every `Err` its inner match produced was dropped where it stood.

import { expect, test } from 'bun:test';
import { BorrowMut } from '@ankurah/base';
import { Inner, Outer, Token, run } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the inner match’s Err leaves the function', () => {
  const out = new BorrowMut('');
  const inner = new Inner('Bad', {});
  const answer = run(new Outer('One', { _0: Token.new(5) }), inner, out);
  inner.drop(); // `run` takes `&Inner`; the temporary is the caller's
  expect(answer.isErr()).toBe(true);
  expect(answer.unwrapErr()).toBe('bad');
  // The arm never reached its own `return`, so nothing was written after 'g'.
  expect(out.value).toBe('');
});

test('the arm still answers when the inner match falls through', () => {
  const out = new BorrowMut('');
  const inner = new Inner('Good', {});
  const answer = run(new Outer('One', { _0: Token.new(5) }), inner, out);
  inner.drop();
  expect(answer.isOk()).toBe(true);
  expect(answer.unwrap()).toBe(5);
  expect(out.value).toBe('g1');
});

test('the arm that takes no payload answers the value after the match', () => {
  const out = new BorrowMut('');
  const inner = new Inner('Good', {});
  const answer = run(new Outer('Two', {}), inner, out);
  inner.drop();
  expect(answer.isOk()).toBe(true);
  expect(answer.unwrap()).toBe(0);
  expect(out.value).toBe('2');
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
