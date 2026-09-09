// Runs the emitted a_returned_write_appends against the runtime.
//
// The string a returned `write!` produces is appended to what the formatter
// already composed, not put in its place.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { large, small } from './input.ts';

test('a write carried out by a return still appends', () => {
  expect(large()).toBe('Size(big)');
  expect(small()).toBe('Size(7)');
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
