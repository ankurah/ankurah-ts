// Runs the emitted keyed_containers against the real runtime. What is under
// test is that every construction builds the RUNTIME container, which hashes a
// key by its `hash()` and compares by its `equals()`. JavaScript's `Map` and
// `Set` compare by identity, so a key rebuilt from the same bytes — which is
// what a key read back off the wire always is — matched nothing.
//
// And `*map.entry(k).or_insert(0) += 1`: `entry` was a method the map did not
// have, and the place was read twice, so the key was cloned twice and the
// second clone leaked.

import { expect, test } from 'bun:test';
import { Bag, Key, built, counted, ordered, tagged } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('from builds a map a rebuilt key can look up', () => {
  const map = built();
  const ask = new Key('a');
  expect(map.get(ask)).toBe(1);
  expect(map.size).toBe(1);
  ask.drop();
  map.drop();
});

test('and a set', () => {
  const set = tagged();
  const ask = new Key('a');
  expect(set.has(ask)).toBe(true);
  ask.drop();
  set.drop();
});

test('an ordered constructor keeps its entries', () => {
  const map = ordered();
  expect(map.size).toBe(1);
  map.drop();
});

test('a derived Default builds the runtime containers', () => {
  const bag = Bag.default();
  const key = new Key('k');
  bag.named.set(key, 1);
  const ask = new Key('k');
  expect(bag.named.get(ask)).toBe(1);
  ask.drop();
  bag.drop();
});

test('entry counts, and reads the place once', () => {
  const words = [new Key('a'), new Key('b'), new Key('a'), new Key('a')];
  const counts = counted(words);
  const a = new Key('a');
  const b = new Key('b');
  expect(counts.get(a)).toBe(3);
  expect(counts.get(b)).toBe(1);
  expect(counts.size).toBe(2);
  a.drop();
  b.drop();
  counts.drop();
  for (const w of words) w.drop();
});

test('nothing leaked and nothing was dropped twice', () => {
  expectNoOwnershipReports();
});
