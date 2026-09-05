// Runs the emitted lifted_exit against the real runtime. What is under test is
// where the exit lands: the Rust says the `?` leaves the whole function, and
// before the sentinel the emitted `?` returned from the arrow the value was
// lifted into. `Result.Err(..)` is a truthy object, so `if (applied)` took the
// SUCCESS branch for a call that had failed — which is what core's
// commit_remote_transaction did with an event it could not apply. A driver that
// only asked for the happy answer would not have seen it, so every function
// here is called on the failing path as well.

import { expect, test } from 'bun:test';
import { ApplyError, Entity, Step, commit, commitBlock, commitEarly, run } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a `?` in the lifted branch of an `if` leaves the function with the error', () => {
  const entity = new Entity('e');
  const answer = commit(entity, false, false);
  expect(answer.isErr()).toBe(true);
  // unwrapErr takes the error out of the Result, so the error is the driver's
  // to release from here on.
  const error = answer.unwrapErr();
  expect(error.type).toBe('Refused');
  error.drop();
  entity.drop();
});

test('and the branch that does not fail still produces the value', () => {
  const entity = new Entity('e');
  expect(commit(entity, false, true).unwrap()).toBe(1);
  // The branch that never asks: `already` is true, so nothing is applied.
  expect(commit(entity, true, false).unwrap()).toBe(1);
  entity.drop();
});

test('a `?` in a block used as a value leaves the function', () => {
  const entity = new Entity('e');
  const failed = commitBlock(entity, false);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
  expect(commitBlock(entity, true).unwrap()).toBe(1);
  entity.drop();
});

test('a plain `return` in a lifted branch is the function\'s return', () => {
  const entity = new Entity('e');
  expect(commitEarly(entity, true, false).unwrap()).toBe(7);
  const failed = commitEarly(entity, false, false);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
  expect(commitEarly(entity, false, true).unwrap()).toBe(1);
  entity.drop();
});

test('an arm of a statement-position match hands its exit back and the caller returns it', () => {
  const entity = new Entity('e');
  const failed = run(entity, new Step('Apply', { _0: false }));
  expect(failed.isErr()).toBe(true);
  const error = failed.unwrapErr();
  expect(error).toBeInstanceOf(ApplyError);
  error.drop();
  expect(run(entity, new Step('Apply', { _0: true })).unwrap()).toBe(1);
  expect(run(entity, new Step('Skip', {})).unwrap()).toBe(0);
  entity.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
