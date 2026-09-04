// Runs the emitted result_values against the real runtime. Two things are under
// test. The fate of every Result the emitter builds: `?` must consume the
// wrapper it tested on both paths, and neither the Ok nor the Err may be left
// behind. And what the error actually is: `width` builds it from the unit
// variant `WireError::Truncated`, and every test below that takes an error out
// READS it — `.type` on a value, not a truthiness check — because the emitter
// used to write that variant as a static the class does not declare, which
// hands back `undefined` and passes every check that only asks whether a
// failure happened.

import { expect, test } from 'bun:test';
import { dropOwned } from '@ankurah/base';
import {
  WireError,
  bound,
  defaulted,
  discarded,
  insideAnExpression,
  width,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('width returns Ok for a non-empty string', () => {
  const result = width('abc');
  expect(result.isOk()).toBe(true);
  expect(result.unwrap()).toBe(3);
});

test('width returns Err for an empty string', () => {
  const result = width('');
  expect(result.isErr()).toBe(true);
  const error = result.unwrapErr();
  expect(error).toBeInstanceOf(WireError);
  expect(error.type).toBe('Truncated');
  error.drop();
});

test('bound unwraps through ? and adds one', () => {
  expect(bound('abcd').unwrap()).toBe(5);
});

test('bound hands the error back out', () => {
  const result = bound('');
  expect(result.isErr()).toBe(true);
  const error = result.unwrapErr();
  expect(error.type).toBe('Truncated');
  error.drop();
});

test('insideAnExpression calls once and lifts the test out of the expression', () => {
  expect(insideAnExpression('ab').unwrap()).toBe(3);
  const failed = insideAnExpression('');
  expect(failed.isErr()).toBe(true);
  dropOwned(failed.unwrapErr());
});

test('discarded releases the Ok it did not want', () => {
  expect(discarded('abc').unwrap()).toBe(0);
  const failed = discarded('');
  expect(failed.isErr()).toBe(true);
  dropOwned(failed.unwrapErr());
});

test('defaulted consumes the Result on both of unwrapOr its paths', () => {
  expect(defaulted('abcde')).toBe(5);
  expect(defaulted('')).toBe(0);
});

test('a WireError clones into a separate value, and equals borrows both', () => {
  const error = new WireError('Truncated', {});
  const copy = error.clone();
  expect(copy.type).toBe('Truncated');
  expect(error.equals(copy)).toBe(true);
  copy.drop();
  error.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
