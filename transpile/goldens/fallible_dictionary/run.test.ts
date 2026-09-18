// Runs the dictionaries the emitted call sites wrote (spec 4.4b): a fallible
// conversion handed over as the `Result` it already answers, an infallible one
// wrapped, and a conversion nothing performs held at its own parameter.

import { expect, test } from 'bun:test';
import { Sel, counted, parsed, refused, shifted } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a fallible concrete site calls the TryFrom impl and reads its Result', () => {
  const sel = parsed();
  expect(sel).toBeInstanceOf(Sel);
  expect(sel!.text).toBe('year >= 2020');
  sel!.drop();
});

test('and the same impl answers Err where Rust does', () => {
  expect(refused()).toBe(null);
});

test('an infallible concrete site still has its answer wrapped', () => {
  const sel = counted();
  expect(sel!.text).toBe('7');
  sel!.drop();
});

test('a conversion no impl performs is a hole that throws', () => {
  // A hole dropped from the argument list rather than held at its own index
  // let the `7i64` after it answer for the parameter before it, and `shifted`
  // returned 7n through a conversion for a type standing nowhere in the call.
  expect(() => shifted()).toThrow();
});

test('nothing leaked', async () => {
  await expectNoOwnershipReports();
});
