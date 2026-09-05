// Runs the emitted await_postfix against the real runtime. Each of the three
// new forms asked the PROMISE for something: the index answered `undefined`,
// and the slice and the call threw `TypeError`.

import { expect, test } from 'bun:test';
import { first, held, tail, through, width } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an index whose base awaits reads the value, not the promise', async () => {
  expect(await first()).toBe(1);
});

test('so does a slice', async () => {
  expect(await tail()).toEqual([2, 3]);
});

test('and a direct call calls what the promise answered', async () => {
  expect(await through()).toBe(16);
});

test('the two forms the fourth pass already covered still work', async () => {
  expect(await width()).toBe(3);
  expect(await held()).toEqual([4, 5]);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
