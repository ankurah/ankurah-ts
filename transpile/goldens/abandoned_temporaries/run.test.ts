// Runs the emitted abandoned_temporaries against the real runtime. What is
// under test is who releases a `?`'s temporary when the statement holding it
// leaves before the `unwrap` that would have consumed it.
//
// Both defective paths are driven: the second `?` returning `Err`, which leaves
// through its own `return`; and the second `?`'s operand panicking, which
// leaves by throwing. On both, Rust drops the `Token` the first `?` produced.
// Against the parent's engine every one of these tests reports a leak.

import { expect, test } from 'bun:test';
import { Refused, Token, both, bothOrPanic, onlyOne, take, three } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('both hands back the two tokens', () => {
  const pair = both(1, 2).unwrap();
  expect(pair[0].n).toBe(1);
  expect(pair[1].n).toBe(2);
  pair[0].drop();
  pair[1].drop();
});

test('the first token is released when the second ? returns Err', () => {
  // `take(0)` is the `Err`, so the second `?` returns and the first `?`'s
  // wrapper — holding a live Token — is never unwrapped. The `finally` is the
  // only thing that releases it.
  const failed = both(1, 0);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
});

test('the first token is released when the second ? operand throws', () => {
  expect(() => bothOrPanic(1, 99)).toThrow('exploding was asked for 99');
});

test('a ? that names its own temporary needs no wrapper release', () => {
  expect(three(1, 2, 3).unwrap()).toBe(6);
  const failed = three(1, 0, 3);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
});

test('the lone ? is unchanged', () => {
  expect(onlyOne(5).unwrap()).toBe(5);
  const failed = onlyOne(0);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
});

test('take hands back its own error', () => {
  const failed = take(0);
  expect(failed.isErr()).toBe(true);
  const error = failed.unwrapErr();
  expect(error).toBeInstanceOf(Refused);
  error.drop();
  const token = take(3).unwrap();
  expect(token).toBeInstanceOf(Token);
  token.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
