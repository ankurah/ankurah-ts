// Runs the emitted option_adaptors against the real runtime.
//
// At the parent every one of these answered a JavaScript sentinel where Rust
// answers `None`: `remove` deleted the last live watcher, `firstLabelOver`
// called `findMap` on an array, `total` raised on an empty vector, and the
// rest handed back `undefined` where the declared type says `null`.

import { expect, test } from 'bun:test';
import { counted, ends, evensBackwards, firstDroppable, firstLabelOver, firstOver, narrowest, Reading, total, Watchers, widest } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

// J1's live case. `findIndex` answered -1 for a watcher that had already gone,
// `-1 != null` read as PRESENT, and `splice(-1, 1)` removed the LAST one.
test('removing a watcher that has already gone leaves the list alone', () => {
  const w = new Watchers([10, 20, 30]);
  w.remove(99);
  expect(w.ids).toEqual([10, 20, 30]);
  w.remove(20);
  expect(w.ids).toEqual([10, 30]);
  w.drop();
});

test('rposition answers the LAST match, and null for none', () => {
  const w = new Watchers([10, 20, 30]);
  expect(w.lastAtLeast(20)).toBe(2);
  expect(w.lastAtLeast(99)).toBe(null);
  w.drop();
});

test('find answers the element or exactly null', () => {
  expect(firstOver([1, 9], 5)).toBe(9);
  const missing = firstOver([1, 2], 99);
  expect(missing).toBe(null);
  // `undefined` would have passed `!= null` and failed this one.
  expect(missing === null).toBe(true);
});

// At the parent this was `_t0.findMap(..)`: a method no array declares.
test('find_map answers what the closure built, or null', () => {
  expect(firstLabelOver(['ab', 'abcd'], 3)).toBe('abcd');
  expect(firstLabelOver(['ab', 'abcd'], 9)).toBe(null);
});

test('first and last answer null for an empty sequence', () => {
  expect(ends([1, 9])).toEqual([1, 9]);
  expect(ends([])).toEqual([null, null]);
});

// `Array.prototype.reduce` with no initial value THROWS on an empty array.
test('reduce answers null for an empty sequence rather than raising', () => {
  expect(total([1, 2, 3])).toBe(6);
  expect(total([])).toBe(null);
});

test('max_by_key keeps the last of a tie and min_by_key the first', () => {
  // Two labels tie at four characters: std keeps the LAST for a maximum and
  // the FIRST for a minimum, and swapping either loop would pass one of these
  // and fail the other.
  const labels = ['aaaa', 'bbbb', 'c'];
  expect(widest(labels)).toBe('bbbb');
  expect(narrowest(['dd', 'e', 'f'])).toBe('e');
  expect(widest([])).toBe(null);
  expect(narrowest([])).toBe(null);
});

// A `.iter()` over a BORROWED vector of droppable elements. At the parent the
// spread was lifted into a temporary with `dropOwned(_t0)` around it, which
// released every element the CALLER still owned: the `r.drop()` below was then
// the second drop and aborted the run.
test('iterating a borrowed sequence releases nothing the caller owns', () => {
  const readings = [new Reading('alpha'), new Reading('beta')];
  expect(firstDroppable(readings, 'be')).toBe(true);
  expect(firstDroppable(readings, 'zz')).toBe(false);
  for (const r of readings) r.drop();
});

// A range is a VALUE in Rust, and the port has no `Range` type: at the parent
// this loop read `for (const n of undefined)` and raised the first time it was
// reached.
test('a range iterates its values, and rev walks them backwards', () => {
  expect(counted(4)).toEqual([0, 1, 2, 3]);
  expect(counted(0)).toEqual([]);
  expect(evensBackwards(6)).toEqual([4, 2, 0]);
  expect(evensBackwards(0)).toEqual([]);
});

test('nothing leaked', async () => {
  await expectNoOwnershipReports();
});
