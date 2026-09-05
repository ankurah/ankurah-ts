// Runs the emitted retain_in_place against the real runtime. At the parent the
// vector was UNCHANGED — the emitter wrote a comment and a `filter` whose answer
// nobody read — and the map was unchanged too, because the `!` tested an arrow
// rather than what the arrow answers.

import { expect, test } from 'bun:test';
import { HashMap } from '@ankurah/base';
import { Bag, Item } from './input.ts';
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

test('nothing leaked and nothing was dropped twice', () => {
  expectNoOwnershipReports();
});
