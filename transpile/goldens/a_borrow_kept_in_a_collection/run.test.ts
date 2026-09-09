// Runs the emitted a_borrow_kept_in_a_collection against the runtime.
//
// Against 8ab991f's engine the constraint erases the `&` on the argument, so
// `picked` reads as a collection of owned `Thing`s: the function releases what
// the caller still owns and `run` releases it a second time.

import { afterAll, expect, test } from 'bun:test';
import { dropOwned } from '@ankurah/base';
import { expectNoOwnershipReports } from './leaks.ts';
import { kept, run, Thing, totalOver } from './input.ts';

test('a collection of borrows releases nothing and the owner releases once', () => {
  expect(run()).toBe(6n);
});

test('a borrowed parameter kept in a collection still belongs to the caller', () => {
  const items = [Thing.new(4n), Thing.new(5n)];
  expect(totalOver(items)).toBe(9n);
  expect(kept(items[0])).toBe(1);
  dropOwned(items);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
