// Runs the emitted slice_copy against the real runtime. At the parent both
// copies were `slice()`/`[...xs]`, which copy the ARRAY and leave both copies
// holding the same elements: the caller dropped what the original still owned,
// and the second drop is a fatal. Here the copy is deep, so both sides can be
// dropped.

import { expect, test } from 'bun:test';
import { Batch, Event } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a copy of a slice is a copy of what it holds', () => {
  const batch = new Batch([new Event(1), new Event(2)]);
  const mine = batch.copyOfEvents();
  expect(mine.map((e) => e.n)).toEqual([1, 2]);
  // Two owners, and each drops its own: at the parent this was one value
  // dropped twice.
  for (const e of mine) e.drop();
  batch.drop();
});

test('`to_owned` is the same copy under another name', () => {
  const events = [new Event(3)];
  const mine = Batch.ownedEvents(events);
  expect(mine[0].n).toBe(3);
  expect(mine[0]).not.toBe(events[0]);
  for (const e of mine) e.drop();
  for (const e of events) e.drop();
});

test('where there is nothing inside to copy, the array copy is the whole copy', () => {
  expect(Batch.copyOfCounts([1, 2, 3])).toEqual([1, 2, 3]);
});

test('nothing leaked and nothing was dropped twice', () => {
  expectNoOwnershipReports();
});
