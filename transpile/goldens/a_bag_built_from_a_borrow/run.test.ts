// Runs the emitted a_bag_built_from_a_borrow against the runtime.
//
// A bag built from a borrowed collection holds borrows: what is drained out of
// it belongs to the caller, and only the caller releases it.

import { afterAll, expect, test } from 'bun:test';
import { dropOwned } from '@ankurah/base';
import { expectNoOwnershipReports } from './leaks.ts';
import { run, Tag, widthsOf } from './input.ts';

test('a bag built from a borrowed collection releases nothing', () => {
  expect(run()).toBe(3);
});

test('the tags a bag was built from still belong to the caller', () => {
  const tags = [Tag.new('aaa'), Tag.new('b')];
  expect(widthsOf(tags)).toBe(4);
  dropOwned(tags);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
