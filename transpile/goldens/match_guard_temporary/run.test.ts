// Runs the emitted match_guard_temporary against the real runtime. Three things
// are under test. The guard reads the name its own pattern bound, so an emitter
// that declared that name after the guard throws a ReferenceError as soon as a
// guarded arm is reached. The `Reading` each guard builds must reach a drop
// whether the guard passed or failed, which the leak registry is what catches.
// And an arm whose guard failed must hand the subject to the arm below it,
// which is what the values these tests expect are checking.

import { expect, test } from 'bun:test';
import { Reading, banded, classify, limitOf } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('limitOf builds a Reading the caller then owns', () => {
  const built: Reading = limitOf(3);
  expect(built.limit).toBe(6);
  built.drop();
});

test('classify takes the literal arm before any guard is evaluated', () => {
  expect(classify(0, 2)).toBe(0);
});

test('classify takes the guarded arm when the guard passes', () => {
  expect(classify(3, 2)).toBe(1);
});

test('classify falls through to the arm below when the guard fails', () => {
  expect(classify(9, 2)).toBe(2);
});

test('banded takes the first guard when it passes', () => {
  expect(banded(1, 2)).toBe(1);
});

test('banded falls from the first guard to the second', () => {
  expect(banded(6, 2)).toBe(2);
});

test('banded falls past both guards to the arm that has none', () => {
  expect(banded(100, 2)).toBe(3);
});

test('both matches survive being walked down every arm many times over', () => {
  for (let value = 0; value < 25; value += 1) {
    classify(value, 2);
    banded(value, 2);
  }
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
