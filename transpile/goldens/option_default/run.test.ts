// Runs the emitted option_default against the real runtime. Two things are under
// test. `unwrap_or` builds its fallback whether or not the option is None, so on
// the Some path the emitted code has to release a fallback nobody took — and it
// has to release exactly that one, never the value it did return. And `?` on an
// Option releases the value it tested when nothing binds it, while handing it to
// the binding when something does.

import { expect, test } from 'bun:test';
import { Entity, borrow, check, make, orElse, orFallback, width } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('make hands a Some payload to the caller and answers null for None', () => {
  const entity = make('abc');
  expect(entity).not.toBeNull();
  expect(entity!.name).toBe('abc');
  entity!.drop();
  expect(make('')).toBeNull();
});

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('ab');
  expect(borrow(entity)).toBe(2);
  entity.drop();
});

test('orFallback returns the option and releases the fallback nobody took', () => {
  const kept = orFallback('kept');
  expect(kept.name).toBe('kept');
  // The value handed back is the option's own, still live and still ours.
  expect(borrow(kept)).toBe(4);
  kept.drop();
});

test('orFallback returns the fallback when the option was None', () => {
  const fallback = orFallback('');
  expect(fallback.name).toBe('fallback');
  fallback.drop();
});

test('orElse builds the fallback only where it is wanted', () => {
  const kept = orElse('kept');
  expect(kept.name).toBe('kept');
  kept.drop();
  const lazy = orElse('');
  expect(lazy.name).toBe('lazy');
  lazy.drop();
});

test('width binds the tested value and releases it at the end of the block', () => {
  expect(width('abcd')).toBe(4);
  expect(width('')).toBeNull();
});

test('check releases the value it tested and wanted nothing from', () => {
  expect(check('abc')).toBe(0);
  expect(check('')).toBeNull();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
