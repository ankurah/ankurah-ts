// Runs the emitted an_unknown_nothing_settles against the runtime.
//
// The record of what an unsolved unknown costs: `counted`'s local is released
// by nobody, because the engine cannot say what it holds and will not guess,
// and the site says so. `filled`'s local, one line different, is released.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { counted, filled } from './input.ts';

test('a body the engine could not type keeps its shape and runs', () => {
  expect(counted(0)).toBe(0);
});

test('the same body with one line that says what it holds releases it', () => {
  expect(filled()).toBe(1);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
