// Runs the emitted assignment_drops against the real runtime. Every assignment
// here overwrites a place that already held something with drop glue, and the
// value that was there has exactly one chance to be released: at the assignment.
// Miss it and the old value is collected unreleased; do it twice and the second
// drop is fatal. The block's own `finally` must then release only what the
// binding holds at the end, not what it held when it started.

import { expect, test } from 'bun:test';
import { Mutex } from '@ankurah/base';
import { Entity, Holder, borrow, maybeReplace, replace, setField, setThroughGuard } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('ab');
  expect(borrow(entity)).toBe(2);
  entity.drop();
});

test('replace releases the first Entity and answers with the second', () => {
  expect(replace('a', 'bbb')).toBe(3);
});

test('maybeReplace releases the first Entity only on the path that overwrote it', () => {
  expect(maybeReplace(true)).toBe(2);
  expect(maybeReplace(false)).toBe(1);
});

test('setField releases the field the struct held and keeps the new one', () => {
  const holder = new Holder(new Entity('old'));
  expect(setField(holder, 'newer')).toBe(5);
  expect(holder.inner.name).toBe('newer');
  // Dropping the Holder cascades into the Entity now in the field, and must not
  // reach the one the assignment already released.
  holder.drop();
});

test('setField twice over releases each Entity exactly once', () => {
  const holder = new Holder(new Entity('one'));
  setField(holder, 'two');
  setField(holder, 'three');
  expect(holder.inner.name).toBe('three');
  holder.drop();
});

test('setThroughGuard replaces what the mutex holds and releases it', () => {
  const cell = new Mutex(new Entity('before'));
  expect(setThroughGuard(cell, 'after')).toBe(5);
  // The mutex is free again, so its guard was released too.
  const guard = cell.lock();
  expect(guard.value.name).toBe('after');
  guard.drop();
  // Dropping the mutex cascades into the Entity now inside it.
  cell.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
