// Runs the emitted literal_match_moves against the real runtime. What is under
// test is the drop flag: the arm that hands the Entity to `consume` must set it,
// so the enclosing `finally` releases the Entity only on the paths that still
// own it. Without the flag the `finally` drops a value `consume` has already
// dropped, and the runtime calls that out as a double drop.

import { expect, test } from 'bun:test';
import { Entity, borrow, byFlag, byNumber, consume } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('consume takes the Entity by value and releases it', () => {
  expect(consume(new Entity('ab'))).toBe(2);
});

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('abc');
  expect(borrow(entity)).toBe(3);
  entity.drop();
});

test('byFlag hands the Entity away on the true arm and keeps it on the false one', () => {
  expect(byFlag(true)).toBe(0);
  expect(byFlag(false)).toBe(0);
});

test('byFlag survives being called many times over', () => {
  for (let i = 0; i < 20; i += 1) {
    expect(byFlag(i % 2 === 0)).toBe(0);
  }
});

test('byNumber releases the Entity on the two arms that kept it', () => {
  expect(byNumber(0)).toBe(0);
  expect(byNumber(2)).toBe(1);
});

test('byNumber hands the Entity away on the arm that consumes it', () => {
  expect(byNumber(1)).toBe(0);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
