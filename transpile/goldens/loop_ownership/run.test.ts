// Runs the emitted loop_ownership against the real runtime. What is under test
// is who releases each element: the turn that borrowed it, the callee that took
// it by value, or the iterator holding whatever the `break` left behind.
//
// A driver never touches an array after handing it to `drain` or `consumeAll`.
// Rust moves the `Vec` into the loop's iterator and the caller cannot name it
// again; here the caller's array survives the call with dropped elements still
// sitting in it, and reading one would be a use-after-drop the source could not
// have written.

import { expect, test } from 'bun:test';
import { Entity, borrow, consume, consumeAll, drain, measure, takeUntil } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('consume takes an Entity by value and releases it', () => {
  expect(consume(new Entity('abc'))).toBe(3);
});

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('abcd');
  expect(borrow(entity)).toBe(4);
  entity.drop();
});

test('drain releases every element it reached and every element it did not', () => {
  expect(drain([new Entity('ab'), new Entity('cd'), new Entity('ef')], 3)).toBe(4);
});

test('drain runs to the end when the break never fires', () => {
  expect(drain([new Entity('ab'), new Entity('cd')], 99)).toBe(4);
});

test('drain over an empty vector reaches neither the body nor a leak', () => {
  expect(drain([], 0)).toBe(0);
});

test('takeUntil releases the element the break left in the body', () => {
  // The `break` stands above the move, so on that path the turn still owns the
  // element it was handed and the drop flag is what says so.
  expect(takeUntil([new Entity('ab'), new Entity('cdef'), new Entity('gh')], 3)).toBe(2);
});

test('takeUntil hands every element on when the break never fires', () => {
  expect(takeUntil([new Entity('ab'), new Entity('cd')], 9)).toBe(4);
});

test('takeUntil breaks on the first turn, releasing that element and the tail', () => {
  expect(takeUntil([new Entity('abcd'), new Entity('e')], 1)).toBe(0);
});

test('drain breaks on the first turn, so the whole tail goes to the iterator', () => {
  expect(drain([new Entity('abcd'), new Entity('e'), new Entity('f')], 1)).toBe(4);
});

test('consumeAll hands each element away, so the body releases nothing', () => {
  expect(consumeAll([new Entity('a'), new Entity('bb'), new Entity('ccc')])).toBe(6);
});

test('measure borrows through the vector and leaves it whole', () => {
  const entities = [new Entity('a'), new Entity('bb')];
  expect(measure(entities)).toBe(3);
  // Still ours: measure released nothing, so both are readable and both are
  // ours to release.
  expect(measure(entities)).toBe(3);
  for (const entity of entities) entity.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
