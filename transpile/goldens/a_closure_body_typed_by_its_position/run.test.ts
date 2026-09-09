// Runs the emitted a_closure_body_typed_by_its_position against the runtime.
//
// Against 9099480's engine the closure's parameter is typed by nothing while
// the body's constraints are collected, so `tags` holds an element the engine
// never named and the tags it built are never released.

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
