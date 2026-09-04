// Runs the emitted question_from against the real runtime. What is under test is
// the conversion `?` performs: the error that leaves a function whose error type
// differs from the one it called must be the *converted* value, built by the
// `From` impl, and the error it was built from must be gone — Rust's `From::from`
// takes its argument by value.
//
// The identity case is here too, because a conversion written where none is
// needed would drop the error and hand on a fresh one nobody asked for.

import { expect, test } from 'bun:test';
import { Wire, Wrapped, doubled, passedOn, read, wrapped } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('read hands back its own error type', () => {
  const failed = read('');
  expect(failed.isErr()).toBe(true);
  const error = failed.unwrapErr();
  expect(error).toBeInstanceOf(Wire);
  expect(error.code).toBe(7);
  error.drop();
});

test('? converts the error where the two types differ', () => {
  const failed = wrapped('');
  expect(failed.isErr()).toBe(true);
  const error = failed.unwrapErr();
  expect(error).toBeInstanceOf(Wrapped);
  expect(error.code).toBe(7);
  expect(error.context).toBe('wire');
  error.drop();
});

test('the Wire the conversion consumed is gone', () => {
  // `Wrapped.fromWire` takes its argument by value, so the `Wire` it was built
  // from is dropped inside it. Nothing here holds one to check directly; the
  // leak check at the end is what would catch it surviving.
  expect(wrapped('abc').unwrap()).toBe(4);
});

test('? writes no conversion where the two types agree', () => {
  const failed = passedOn('');
  expect(failed.isErr()).toBe(true);
  const error = failed.unwrapErr();
  expect(error).toBeInstanceOf(Wire);
  error.drop();
});

test('a ? whose value the position names carries it through', () => {
  expect(doubled('ab').unwrap()).toBe(6);
  const failed = doubled('');
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
