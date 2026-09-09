// Runs the emitted a_collection_typed_below against the real runtime.
//
// A collection's element comes from the `push` BELOW its `let`, so the vector
// is released on both ways out.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { collectEntities, countEntities, firstId } from './input.ts';

test('a collection built by a loop carries the element the loop pushes', () => {
  const entities = collectEntities([7n, 8n]);
  expect(entities.length).toBe(2);
  expect(entities[0].id).toBe(7n);
  dropAll(entities);
});

test('a collection the caller does not keep is released where it was built', () => {
  expect(countEntities([1n, 2n, 3n])).toBe(3);
  expect(firstId([3n])).toBe(3n);
});

function dropAll(entities: { drop(): void }[]) {
  for (const entity of entities) entity.drop();
}

afterAll(async () => {
  await expectNoOwnershipReports();
});
