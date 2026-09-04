// Runs the emitted conversions against the real runtime. Two things are under
// test. That each conversion reaches the right function: `.into()` and
// `Target::from(..)` both land on the static the `From` impl was emitted as, and
// the value they consumed is gone afterwards. And that `as` produces the value
// Rust produces: a widening into a 64-bit integer crosses into `bigint`, a
// narrowing keeps the low bits, and a float truncates towards zero.

import { expect, test } from 'bun:test';
import { Name, Tag, fromCall, named, narrow, owned, truncate, widen } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('.into() reaches the From impl and consumes what it converted', () => {
  const name = named(new Tag(12));
  expect(name).toBeInstanceOf(Name);
  expect(name.text).toBe('12');
  name.drop();
});

test('Target::from(..) reaches the same impl', () => {
  const name = fromCall(new Tag(34));
  expect(name.text).toBe('34');
  name.drop();
});

test('to_string on a string is the string', () => {
  expect(owned('abc')).toBe('abc');
});

test('a widening into a 64-bit integer is a bigint', () => {
  expect(widen(7)).toBe(7n);
  expect(typeof widen(7)).toBe('bigint');
});

test('a narrowing keeps the low 32 bits', () => {
  expect(narrow(7n)).toBe(7);
  // 2^32 + 5 keeps only the 5, as Rust's `as` does.
  expect(narrow(4294967301n)).toBe(5);
  expect(typeof narrow(7n)).toBe('number');
});

test('a float truncates towards zero on its way to an integer', () => {
  expect(truncate(3.9)).toBe(3);
  expect(truncate(-3.9)).toBe(-3);
});

test('a float outside the range saturates rather than wrapping', () => {
  // Rust answers i32::MAX here; masking the low bits of the truncated double
  // answered an arbitrary number.
  expect(truncate(1e30)).toBe(2147483647);
  expect(truncate(-1e30)).toBe(-2147483648);
});

test('a NaN becomes zero, as Rust says', () => {
  expect(truncate(Number.NaN)).toBe(0);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
