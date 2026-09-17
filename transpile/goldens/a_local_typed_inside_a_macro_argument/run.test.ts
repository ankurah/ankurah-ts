// Runs the emitted a_local_typed_inside_a_macro_argument against the runtime.
//
// Each `Tag` a macro argument put into a local is released once, by the local.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { logged, printed, run } from './input.ts';

test('a local a log line binds releases what it holds', () => {
  expect(logged()).toBe(1);
});

test('a local a print line binds releases what it holds', () => {
  expect(printed()).toBe(1);
});

test('both together', () => {
  expect(run()).toBe(2);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
