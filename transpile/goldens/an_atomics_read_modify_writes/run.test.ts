// Runs the emitted an_atomics_read_modify_writes against the runtime.
//
// Each call answers the value it found and leaves the operator's answer behind.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { bitwiseOverALocal, boundsOverALocal, logicOverALocal, throughAField } from './input.ts';

test('fetch_or, fetch_and and fetch_xor answer the old bits', () => {
  expect(bitwiseOverALocal()).toEqual([12, 15, 6, 3]);
});

test('fetch_max and fetch_min answer the old value and keep the bound', () => {
  expect(boundsOverALocal()).toEqual([5, 9, 9, 2]);
});

test('an AtomicBool takes the logical operators', () => {
  expect(logicOverALocal()).toEqual([true, false, true, false]);
});

test('a field is the same place its owner writes', () => {
  expect(throughAField()).toEqual([10, 11]);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
