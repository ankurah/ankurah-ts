// Runs the emitted value_positions against the real runtime. Rust reads four
// things here as VALUES that TypeScript writes as statements: a `loop` with a
// `break n`, a block whose one statement is an `if`, a jump where a ternary
// branch would stand, and the tail of a `for` body. Three of the four did not
// parse at all; the fourth, the loop tail, parsed and was WRONG — the block
// translator put a `return` in front of it, so the loop left on its first turn
// and the `?` inside an arm answered a bare `Result.Err` as the function's
// value.

import { expect, test } from 'bun:test';
import { firstEven, pick, total, untilZero } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a loop standing where a value is wanted answers what it broke with', () => {
  expect(firstEven(3)).toBe(5);
  expect(firstEven(4)).toBe(5);
});

test('a block of one `if` is the value that `if` produces', () => {
  expect(pick(true)).toBe(2);
  expect(pick(false)).toBe(3);
});

test('a break written where a ternary branch would stand leaves the loop', () => {
  expect(untilZero([1, 2, 3])).toBe(6);
  expect(untilZero([1, 0, 3])).toBe(1);
});

test('a for body runs every turn, and an arm’s `?` leaves the function', () => {
  const answer = total([1, 2, 3]);
  expect(answer.isOk()).toBe(true);
  expect(answer.unwrap()).toBe(6);
});

test('the whole loop runs, not only its first turn', () => {
  const answer = total([0, 0, 0]);
  expect(answer.isOk()).toBe(true);
  expect(answer.unwrap()).toBe(3);
});

test('nothing leaked and nothing was dropped twice', () => {
  expectNoOwnershipReports();
});
