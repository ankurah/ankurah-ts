// Runs the emitted a_borrow_kept_in_a_collection against the runtime.
//
// A collection of borrows releases nothing: the `Thing`s belong to the caller,
// and each is released once, by the caller.

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
