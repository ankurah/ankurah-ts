// Runs the emitted retain_in_place against the real runtime. At the parent the
// vector was UNCHANGED — the emitter wrote a comment and a `filter` whose answer
// nobody read — and the map was unchanged too, because the `!` tested an arrow
// rather than what the arrow answers.

import { expect, test } from 'bun:test';
import { HashMap } from '@ankurah/base';
import { Bag, Gate, Item } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a vector keeps what the predicate accepts, in place', () => {
  const bag = new Bag([new Item(1), new Item(5), new Item(3)], new HashMap<string, boolean>());
  bag.keepOver(3);
  expect(bag.items.map((i) => i.n)).toEqual([5, 3]);
  // and the ones it passed over are dropped, not leaked: `expectNoOwnershipReports`
  // below is what says so.
  bag.drop();
});

test('a map deletes what the predicate rejects', () => {
  const flags = new HashMap<string, boolean>();
  flags.set('on', true);
  flags.set('off', false);
  const bag = new Bag([], flags);
  bag.keepSet();
  expect(bag.flags.size).toBe(1);
  expect(bag.flags.get('on')).toBe(true);
  expect(bag.flags.has('off')).toBe(false);
  bag.drop();
});

test('a captured predicate is built once and released once', () => {
  // At the parent the predicate was interpolated inside the loop, so this
  // `OwnedClosure` was constructed once per element and threw `TypeError: ...
  // is not a function` on the first — and its capture was never dropped.
  const bag = new Bag([new Item(1), new Item(5), new Item(3)], new HashMap<string, boolean>());
  bag.keepOverGate(new Gate(3));
  expect(bag.items.map((i) => i.n)).toEqual([5, 3]);
  bag.drop();
});

test('a predicate that throws leaves the vector valid', () => {
  // Rust's own `retain` guard: what the predicate accepted stays, what it
  // rejected is gone, the element it threw on is counted unprocessed and kept,
  // and so is everything after it. At the parent the array was truncated only
  // on normal completion, so the dropped elements were still in it and the kept
  // ones were duplicated — a later cascade dropped them twice.
  const bag = new Bag([new Item(5), new Item(1), new Item(0), new Item(7)], new HashMap<string, boolean>());
  expect(() => bag.keepUntilZero()).toThrow('zero');
  // 5 accepted; 1 rejected and dropped; 0 threw and is kept; 7 never reached.
  expect(bag.items.map((i) => i.n)).toEqual([5, 0, 7]);
  bag.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
