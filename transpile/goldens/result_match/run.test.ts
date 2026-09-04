// Runs the emitted result_match against the real runtime. A `match` on a call
// that returns Result is a by-value match, and the emitter writes it as
// isOk()/unwrap()/unwrapErr(): isOk borrows, and whichever unwrap runs takes the
// payload out and leaves the Result moved. So the Result is never dropped and
// is never a leak, and each arm answers for the payload it was handed — handed
// on, released where it stands, or handed to the caller.

import { expect, test } from 'bun:test';
import {
  Entity,
  Failure,
  borrowEntity,
  borrowFailure,
  consumeEntity,
  consumeFailure,
  fetch,
  orDefault,
  score,
  width,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('fetch builds an Ok whose payload is the caller to release', () => {
  const result = fetch('abc');
  expect(result.isOk()).toBe(true);
  const entity = result.unwrap();
  expect(entity.name).toBe('abc');
  entity.drop();
});

test('fetch builds an Err whose payload is the caller to release', () => {
  const result = fetch('');
  expect(result.isErr()).toBe(true);
  const failure = result.unwrapErr();
  expect(failure.reason).toBe('empty');
  failure.drop();
});

test('consumeEntity and consumeFailure each take a payload by value', () => {
  expect(consumeEntity(new Entity('ab'))).toBe(2);
  expect(consumeFailure(new Failure('xyz'))).toBe(3);
});

test('borrowEntity and borrowFailure leave the payload to its owner', () => {
  const entity = new Entity('abcd');
  const failure = new Failure('zz');
  expect(borrowEntity(entity)).toBe(4);
  expect(borrowFailure(failure)).toBe(2);
  entity.drop();
  failure.drop();
});

test('width hands each payload to a callee on both arms', () => {
  expect(width('abcde')).toBe(5);
  expect(width('')).toBe(5);
});

test('score keeps each payload in its arm, so each arm releases it', () => {
  expect(score('abc')).toBe(4);
  expect(score('')).toBe(105);
});

test('orDefault hands the Ok payload out and releases the Err payload', () => {
  const kept = orDefault('kept');
  expect(kept.name).toBe('kept');
  kept.drop();
  const fallback = orDefault('');
  expect(fallback.name).toBe('fallback');
  fallback.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
