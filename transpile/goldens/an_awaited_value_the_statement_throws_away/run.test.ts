// Runs the emitted an_awaited_value_the_statement_throws_away.
//
// Both `Held`s are released: the one the statement discards, and the one read.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { discardsWhatItAwaited } from './input.ts';

test('a discarded awaited value is released', async () => {
  expect(await discardsWhatItAwaited()).toBe(2n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
