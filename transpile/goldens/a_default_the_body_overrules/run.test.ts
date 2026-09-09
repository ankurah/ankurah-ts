// Runs the emitted a_default_the_body_overrules against the runtime.
//
// A declaration's default is applied after the arguments are read, so `B` is
// what the body says and `s.b.wrapping_add(100)` resolves on it.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { held, run, wrapped } from './input.ts';

test('a type argument the source left off is what the arguments say, not the default', () => {
  expect(held()).toBe(10n);
  expect(wrapped()).toBe(44);
  expect(run()).toBe(56n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
