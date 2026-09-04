// Runs the emitted macro_moves against the real runtime. Two macros, two
// opposite answers to the same question. `vec!` takes its elements by value, so
// `gather` must emit no drop for either local and the Batch it returns is what
// owns them; a `finally` there would release values the caller then reads.
// `format!` takes its arguments by reference, so `describe` must release both
// locals; no drop there and the leak registry finds them.

import { expect, test } from 'bun:test';
import { Batch, Entity, borrow, describe, gather } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('abc');
  expect(borrow(entity)).toBe(3);
  entity.drop();
});

test('gather hands both locals to the Batch it returns', () => {
  const batch = gather();
  expect(batch.entities.length).toBe(2);
  // Both are live: gather released neither.
  expect(borrow(batch.entities[0]!)).toBe(1);
  expect(borrow(batch.entities[1]!)).toBe(2);
  // Dropping the Batch cascades through the vector into both.
  batch.drop();
});

test('gather twice over builds two independent Batches', () => {
  const one = gather();
  const two = gather();
  expect(one.entities[0]).not.toBe(two.entities[0]);
  one.drop();
  two.drop();
});

test('describe reads both locals and releases both', () => {
  expect(describe()).toBe('a:2');
});

test('describe can run twice over, so neither local outlives its block', () => {
  describe();
  expect(describe()).toBe('a:2');
});

test('a Batch built here owns its vector the same way', () => {
  const batch = new Batch([new Entity('x'), new Entity('yy')]);
  expect(batch.entities.length).toBe(2);
  batch.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
