// Runs the emitted borrowed_iteration against the real runtime. What is under
// test is who owns the elements a `for` loop hands out. Rust decides that with
// the `IntoIterator` impl it selected, and the written `&` is the only thing
// telling `IntoIterator for HashMap<K, V>` (Item = (K, V), the loop's to
// release) from `IntoIterator for &HashMap<K, V>` (Item = (&K, &V), the map's).
// With the `&` erased, `sumAmp` released the borrowed key and value on every
// turn and then the map released them again — the strict registry aborts that
// with `BUG: Key was dropped twice`.

import { expect, test } from 'bun:test';
import { HashMap } from '@ankurah/base';
import { bumpCells, bumpMap, Cell, Key, Ordering, firstWidth, orderingWidth, refWidths, refWidthsBorrowed, sumAmp, sumBorrowed, sumConsuming, widths, widthsViaCall } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

function filled(): HashMap<Key, Cell> {
  const map = new HashMap<Key, Cell>();
  map.set(new Key('a'), new Cell(2));
  map.set(new Key('bb'), new Cell(40));
  return map;
}

test('a map reached through a reference parameter is only read', () => {
  const map = filled();
  expect(sumBorrowed(map)).toBe(42);
  // Every key and value survived the loop, so a second pass answers the same.
  expect(sumBorrowed(map)).toBe(42);
  map.drop();
});

test('`for .. in &map` reads the map and then the map releases it once', () => {
  expect(sumAmp(filled())).toBe(42);
});

test('`for .. in map` takes each key and value out', () => {
  expect(sumConsuming(filled())).toBe(42);
});

test('a borrowed vec keeps its elements', () => {
  const keys = [new Key('a'), new Key('bb')];
  expect(widths(keys)).toBe(3);
  expect(widths(keys)).toBe(3);
  for (const key of keys) key.drop();
});

test('a vec taken by value releases the tail the loop never reached', () => {
  expect(firstWidth([new Key('a'), new Key('bb')])).toBe(1);
});

test('an if-let over a reference leaves the vector to the field', () => {
  const ordering = new Ordering([new Key('a'), new Key('bb')]);
  expect(orderingWidth(ordering)).toBe(3);
  // The field still holds its keys, so a second read answers the same.
  expect(orderingWidth(ordering)).toBe(3);
  ordering.drop();
});

// An explicit `ref` binding over an OWNED sequence: Rust's `IntoIter` hands out
// one element per turn and drops it at the end of that turn. The binding's own
// type is a `&Key`, which owns nothing, so nothing released the element — and
// the tail release starts after the current index, so it could not reach one
// the turn had already handed out. Every element leaked.
test('a ref binding over an owned vec releases each element', () => {
  expect(refWidths([new Key('a'), new Key('bb')])).toBe(3);
});

test('and over a reference it releases nothing, because it owns nothing', () => {
  const keys = [new Key('a'), new Key('bb')];
  expect(refWidthsBorrowed(keys)).toBe(3);
  // The caller still holds them, so a second read answers the same.
  expect(refWidthsBorrowed(keys)).toBe(3);
  for (const key of keys) key.drop();
});

// E11: `(&keys).into_iter()` is `IntoIterator for &Vec<T>`, whose Item is a
// BORROW. The parent engine erased the `&` before the probe ran, picked the
// by-value impl, and released every element the caller still owns — so the
// `k.drop()` below was the second drop and aborted the run.
test('a borrow written as a call still owns nothing', () => {
  // The vector is the function's, and the LOOP borrows it: the block releases
  // each element once, at the end. At the parent the loop released them too,
  // so `dropOwned(keys)` was the second drop and aborted the run.
  expect(widthsViaCall([new Key('a'), new Key('bb')])).toBe(3);
});

// F4/E12: `iter_mut` had no lowering and emitted `cells.iterMut()`, a method no
// array declares. It writes THROUGH to the caller's elements, and releases
// none of them.
test('iter_mut writes through and releases nothing', () => {
  const cells = [new Cell(1), new Cell(2)];
  bumpCells(cells);
  expect(cells.map((c) => c.value)).toEqual([2, 3]);
  bumpCells(cells);
  expect(cells.map((c) => c.value)).toEqual([3, 4]);
  for (const c of cells) c.drop();

  const map = filled();
  bumpMap(map);
  expect([...map].map(([, c]) => c.value).sort((a, b) => a - b)).toEqual([3, 41]);
  map.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  // The recorded leak is gone: `sum_consuming`'s `for (k, v) in map` now goes
  // through `intoEntries()`, which empties the map and marks it dropped, so
  // there is no container left for the collector to find.
  await expectNoOwnershipReports();
});
