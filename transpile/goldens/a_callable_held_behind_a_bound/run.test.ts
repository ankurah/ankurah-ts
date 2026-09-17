// Runs the emitted a_callable_held_behind_a_bound against the runtime.
//
// An Arc is not a function, and a `u64` argument is a bigint.

import { afterAll, expect, test } from 'bun:test';
import { Arc } from '@ankurah/base';
import { expectNoOwnershipReports } from './leaks.ts';
import { heldBehindABound, throughABareName } from './input.ts';

test('a callable held behind a bound is called through the helper', () => {
  expect(heldBehindABound()).toBe(42n);
});

test('a bare name holding a callable takes the argument type it declares', () => {
  expect(throughABareName(Arc.new((n: bigint) => n + 1n))).toBe(5n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
