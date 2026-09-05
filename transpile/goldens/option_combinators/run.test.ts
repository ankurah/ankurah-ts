// Runs the emitted option_combinators against the real runtime. Each of these
// answered wrongly at the parent: `take` removed the entry, discarded it,
// removed nothing the second time and answered `Err` — with the removed entry
// leaked; `weightless` answered `true` for an id the registry does not hold,
// because the comparison it was written beside was swallowed by the ternary's
// false branch.

import { expect, test } from 'bun:test';
import { Entry, Registry } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a receiver with an effect runs once, and its value is what comes back', () => {
  const r = Registry.new();
  r.put(1, 5);
  const taken = r.take(1);
  expect(taken.isOk()).toBe(true);
  const entry = taken.unwrap();
  expect(entry.weight).toBe(5);
  entry.drop();
  // The entry is gone because `remove` ran once — not removed, discarded, and
  // removed again.
  const missing = r.take(1);
  expect(missing.isErr()).toBe(true);
  missing.drop();
  expect(r.calls).toBe(2);
  r.drop();
});

test('a ternary written beside a comparison is what the comparison reads', () => {
  const r = Registry.new();
  r.put(1, 5);
  r.put(2, 0);
  expect(r.weightless(1)).toBe(false);
  expect(r.weightless(2)).toBe(true);
  // The `None` case: `map_or(0, ..) == 0` is true because the DEFAULT is zero,
  // and it used to be true because `0 === 0` was the whole false branch.
  expect(r.weightless(99)).toBe(true);
  r.drop();
});

test('the reading combinators answer what Rust answers', () => {
  const r = Registry.new();
  r.put(1, 5);
  r.put(2, 1);
  expect(r.weightOf(1)).toBe(5);
  expect(r.weightOf(99)).toBe(null);
  expect(r.heavyWeight(1)).toBe(5);
  expect(r.heavyWeight(2)).toBe(null);
  expect(r.heavyWeight(99)).toBe(null);
  expect(r.isHeavy(1)).toBe(true);
  expect(r.isHeavy(2)).toBe(false);
  expect(r.isHeavy(99)).toBe(false);
  expect(r.weightOrFail(1).unwrap()).toBe(5);
  expect(r.weightOrFail(99).unwrapErr()).toBe('no 99');
  r.drop();
});

test("ok_or's error is built before the branch and released where it is not used", () => {
  const r = Registry.new();
  r.put(1, 5);
  // The `Ok` path: the spare error was built — Rust builds it too — and the
  // branch that hands it nowhere releases it. Nothing here can release it, so
  // a leak is the transpiler's.
  const taken = r.takeOrSpare(1, 9);
  expect(taken.isOk()).toBe(true);
  const entry = taken.unwrap();
  expect(entry.weight).toBe(5);
  entry.drop();
  // The `Err` path: the spare is what comes back, and it is the driver's now.
  const missing = r.takeOrSpare(99, 9);
  expect(missing.isErr()).toBe(true);
  const spare = missing.unwrapErr();
  expect(spare.weight).toBe(9);
  spare.drop();
  r.drop();
});

test("map_or's default is released on the path that runs the closure", () => {
  const r = Registry.new();
  r.put(1, 5);
  // `Some`: the closure's value comes back and the default the call was handed
  // is released, because Rust drops it there.
  const mapped = r.entryOr(1, new Entry(9));
  expect(mapped.weight).toBe(5);
  mapped.drop();
  // `None`: the default IS the answer.
  const fallback = r.entryOr(99, new Entry(9));
  expect(fallback.weight).toBe(9);
  fallback.drop();
  r.drop();
});

// R10: Rust drops the default AFTER the closure has run, and on a panic as well
// — so the release is a `finally` around the call rather than a comma in front
// of it. A comma BEHIND the call would get the order right and abandon the
// default whenever the closure throws, which is what this asserts.
test("map_or's default is released even when the closure panics", () => {
  const r = Registry.new();
  r.put(1, 0); // a weight of 0: `checked_sub(1)` is `None` and `expect` panics
  const spare = new Entry(9);
  expect(() => r.entryOrStrict(1, spare)).toThrow('weight underflow');
  expect(spare.isDropped).toBe(true);
  r.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
