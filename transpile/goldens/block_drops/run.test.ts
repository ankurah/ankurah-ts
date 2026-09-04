// Runs the emitted block_drops against the real runtime. What is under test is
// the `finally` the emitter wrote: both locals must be released whichever way
// the body leaves, and the block that owns nothing must not try to release
// anything.

import { expect, test } from 'bun:test';
import { Entity, Registry } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('describe releases both locals on the early return', () => {
  const registry = Registry.new();
  expect(registry.describe(true)).toBe(0);
  registry.drop();
});

test('describe releases both locals on the fall-through', () => {
  const registry = Registry.new();
  expect(registry.describe(false)).toBe(0);
  registry.drop();
});

test('describe can run twice over, so neither local outlives its block', () => {
  const registry = Registry.new();
  registry.describe(true);
  registry.describe(false);
  expect(registry.describe(true)).toBe(0);
  registry.drop();
});

test('tally adds a Copy local to the count field and owns nothing', () => {
  const registry = new Registry(4);
  expect(registry.tally()).toBe(7);
  registry.drop();
});

test('an Entity is a Struct the caller releases', () => {
  const entity = new Entity('abc');
  expect(entity.name).toBe('abc');
  entity.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
