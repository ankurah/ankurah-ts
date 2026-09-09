// Runs the emitted two_unknowns_at_one_type_parameter against the runtime.
//
// The collection the sibling typed holds the Tag moved into it, and releases it.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { moved } from './input.ts';

test('two collections at one type parameter say what each other holds', () => {
  expect(moved()).toBe(1);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
