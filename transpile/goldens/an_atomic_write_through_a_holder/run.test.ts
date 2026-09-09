// Runs the emitted an_atomic_write_through_a_holder against the runtime.
//
// The read through the holder's accessor IS the value and answers. The write
// through it cannot land, so the emitter writes a hole and the runtime refuses
// the call rather than adding to a copy nobody reads back.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { bump, readBack } from './input.ts';

test('a read through the holder answers with what it holds', () => {
  expect(readBack()).toBe(7);
});

test('a write through the holder refuses rather than landing on a copy', () => {
  expect(() => bump()).toThrow();
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
