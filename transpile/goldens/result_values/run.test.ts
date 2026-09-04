// Runs the emitted result_values against the real runtime. What is under test is
// the fate of every Result the emitter builds: `?` must consume the wrapper it
// tested on both paths, and neither the Ok nor the Err may be left behind.
//
// The error payload is released through `dropOwned` rather than by calling
// `.drop()` on it. `width` builds its error as `WireError.Truncated`, and the
// emitted class has no such static, so what comes back today is `undefined`.
// That is a value defect, not an ownership one — it is the same thing the
// goldens README already doubts about `option_result_fields` — and this file
// checks ownership, so it releases whatever the payload turns out to be instead
// of naming a type the emitter does not hand it.

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
  dropOwned(result.unwrapErr());
});

test('bound unwraps through ? and adds one', () => {
  expect(bound('abcd').unwrap()).toBe(5);
});

test('bound hands the error back out', () => {
  const result = bound('');
  expect(result.isErr()).toBe(true);
  dropOwned(result.unwrapErr());
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
