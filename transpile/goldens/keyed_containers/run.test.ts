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
import { Bag, Key, Lists, built, counted, ordered, tagged } from './input.ts';
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

// The three ways of finishing an entry that the counter above does not use.
// Each of these raised `TypeError: ....push is not a function` at the parent:
// the finisher answers the write-through `Slot`, and the emitted code used it
// as the value it points at.
test('or_default reads the place as a value', () => {
  const l = Lists.new();
  l.pushDefault(new Key('a'), 1);
  l.pushDefault(new Key('a'), 2);
  const ask = new Key('a');
  expect(l.count(ask)).toBe(2);
  ask.drop();
  l.drop();
});

test('or_insert and or_insert_with read the place as a value', () => {
  const l = Lists.new();
  l.pushInsert(new Key('a'), 1);
  l.pushWith(new Key('a'), 2);
  l.pushWith(new Key('b'), 3);
  const a = new Key('a');
  const b = new Key('b');
  expect(l.count(a)).toBe(2);
  expect(l.count(b)).toBe(1);
  a.drop();
  b.drop();
  l.drop();
});

// A `BTreeMap` receiver: the value type an `or_default()` needs a thunk for was
// read off `hash_map::Entry` alone, so this one emitted `orDefault()` and
// invoked `undefined` on the first unseen key.
test('a BTreeMap entry gets its thunk too', () => {
  const l = Lists.new();
  l.pushOrdered('x', 1);
  l.pushOrdered('x', 2);
  l.pushOrdered('y', 3);
  expect(l.orderedCount('x')).toBe(2);
  expect(l.orderedCount('y')).toBe(1);
  expect(l.orderedCount('z')).toBe(0);
  l.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
