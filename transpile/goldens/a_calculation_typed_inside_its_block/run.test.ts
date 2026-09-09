// Runs the emitted a_calculation_typed_inside_its_block against the runtime.
//
// The calculation answers what its closure answers, so the function that reads
// it returns a value at all, and the reader the block prepared is released.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { doubled } from './input.ts';

test('a closure prepared in a block says what the calculation holds', () => {
  expect(doubled()).toBe(6n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
