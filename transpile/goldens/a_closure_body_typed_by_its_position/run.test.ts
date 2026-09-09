// Runs the emitted a_closure_body_typed_by_its_position against the runtime.
//
// The closure's parameter comes from the position it stands in, so the
// collection its body fills holds an element the engine can name and the tags
// it built are released.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { Bus, Entity, tagLengthsThroughAFunction, tagLengthsThroughAMethod } from './input.ts';

test('a closure at a method position types the locals in its body', () => {
  const bus = new Bus([new Entity(1n), new Entity(2n)]);
  expect(tagLengthsThroughAMethod(bus)).toBe(2);
  bus.drop();
});

test('a closure at a free function position types them too', () => {
  const entities = [new Entity(3n)];
  expect(tagLengthsThroughAFunction(entities)).toBe(1);
  for (const entity of entities) entity.drop();
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
