// Runs the emitted closure_typing against the real runtime. What is under test
// is that the emitted closures actually do their work: their parameters are
// typed from the position each closure stands in, so the calls inside their
// bodies are the ones the impl table resolved rather than names dispatched on
// spec. A body that had fallen back would still parse and would answer wrongly
// or throw here.

import { expect, test } from 'bun:test';
import { Reading, counted, eachDoubled, scaled, threshold } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

function readings(...levels: number[]): Reading[] {
  return levels.map((level) => new Reading(level));
}

function release(values: Reading[]): void {
  for (const value of values) value.drop();
}

test('the callee bound types the closure parameter, so the method inside it resolves', () => {
  const values = readings(1, 2, 3);
  expect(eachDoubled(values)).toEqual([2, 4, 6]);
  release(values);
});

test('the closure own annotation types the parameter', () => {
  const values = readings(4, 5);
  expect(scaled(values)).toEqual([4, 5]);
  release(values);
});

test('a boxed callable at the return position types the closure it holds', () => {
  const over = threshold(3);
  expect(over(4)).toBe(true);
  expect(over(2)).toBe(false);
});

test('a closure taking a reference reads through it', () => {
  const values = readings(0, 1, 2, 0);
  expect(counted(values)).toBe(2);
  release(values);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
