// Runs the emitted partial_move against the real runtime. What is under test is
// what `takeField` does to the struct it takes from: the field's value belongs
// to the caller from there, the cascade stops releasing it, and the struct is
// still the block's to drop — so `pair.drop()` in the `finally` must release
// only what `pair` still holds and must not double-drop what moved out.
//
// This driver never reads a field after it moved out. That read is fatal by
// design — it is the read Rust would have rejected — and a driver that provoked
// it would record a fatal that the ownership check at the end then reports.

import { expect, test } from 'bun:test';
import { Entity, Pair, Single, borrow, consume, nameOfOne, split, takeBoth, takeOne, widthOfOne } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('consume takes an Entity by value and releases it', () => {
  expect(consume(new Entity('ab'))).toBe(2);
});

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('abc');
  expect(borrow(entity)).toBe(3);
  entity.drop();
});

test('takeOne moves one field out and drops the Pair holding the other', () => {
  expect(takeOne(new Pair(new Entity('ab'), new Entity('cde')))).toBe(5);
});

test('split hands the moved field to the Single it returns', () => {
  const single = split(new Pair(new Entity('kept'), new Entity('gone')));
  expect(single.only.name).toBe('kept');
  // Dropping the Single cascades into the Entity it now owns.
  single.drop();
});

test('takeBoth empties the Pair, and dropping it cascades into nothing', () => {
  expect(takeBoth(new Pair(new Entity('a'), new Entity('bb')))).toBe(3);
});

test('a Pair nobody takes from releases both fields', () => {
  const pair = new Pair(new Entity('x'), new Entity('yy'));
  expect(borrow(pair.one)).toBe(1);
  expect(borrow(pair.two)).toBe(2);
  pair.drop();
});

test('a Single built here owns its Entity the same way', () => {
  const single = new Single(new Entity('only'));
  expect(single.only.name).toBe('only');
  single.drop();
});

// A method declared `self` takes its receiver with it, and a receiver that is a
// FIELD of a place is a partial move. Written as a plain property read, `pair`
// still owned what `intoName` had taken: the `pair.drop()` the emitted body
// writes was then a second drop of the same Entity, and the strict registry
// aborts with `BUG: Entity was used after being moved`. Six of ankql's seven
// `ast.test.ts` failures are exactly this shape.
test('a self method on a field takes the field out of the struct', () => {
  expect(nameOfOne(new Pair(new Entity('a'), new Entity('bb')))).toBe('a');
});

// A `&self` method takes nothing, and the field is read where it is.
test('a borrowing method on a field takes nothing', () => {
  const pair = new Pair(new Entity('a'), new Entity('bb'));
  expect(widthOfOne(pair)).toBe(1);
  expect(widthOfOne(pair)).toBe(1);
  pair.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
