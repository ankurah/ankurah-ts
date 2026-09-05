// Runs the emitted consts_and_literals against the real runtime. Two claims:
// a module-level `const` and a `static` carry their VALUE, and a struct literal
// hands each value to the field it was written beside.

import { expect, test } from 'bun:test';
import { FLOOR, Rec, SYSTEM_COLLECTION, TAG_NULL, TAG_STRING, WORDS, arm, bump, collection, epsilonNear, movedOrigin, ordered, radix, shifted, widths, word, wrapAround } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a const carries its value, not `undefined`', () => {
  expect(TAG_NULL).toBe(0);
  expect(TAG_STRING).toBe(4);
  expect(WORDS).toEqual(['ack', 'alabama', 'alanine']);
  // The defect this pins: `WORDLIST[x]` on `undefined as any` throws, and
  // `humanize` is the function that does it.
  expect(word(1)).toBe('alabama');
});

test('a `static` is an item at all, and carries its value', () => {
  expect(SYSTEM_COLLECTION).toBe('_ankurah_system');
  expect(collection()).toBe('_ankurah_system');
});

test('a const expression is evaluated, bigint width and all', () => {
  expect(shifted()).toBe(1099511627776n);
});

test('a struct literal hands each value to the field it was written beside', () => {
  // Written `third, first, second`; the constructor takes `first, second, third`.
  const rec = Rec.make(7, 'hello', true);
  expect(rec.first).toBe(7);
  expect(rec.second).toBe('hello');
  expect(rec.third).toBe(true);
  expect(rec.tag()).toBe(TAG_STRING);
  rec.drop();
});

test('each use of a non-Copy const is its own value', () => {
  // `first.x = 9` mutates a value of its own; `second` is another `ORIGIN`.
  expect(movedOrigin()).toBe(9);
  // And the module name is not a value anything can mutate or release: a second
  // call answers the same.
  expect(movedOrigin()).toBe(9);
});

test('a static with interior mutability is written through', () => {
  expect(bump()).toBe(0);
  expect(bump()).toBe(1);
  expect(arm(true)).toBe(true);
  expect(arm(false)).toBe(false);
});

test('a negated literal in a const keeps its width', () => {
  expect(FLOOR).toBe(-9007199254740991n);
});

test('a const in a pattern is a comparison, not a binding', () => {
  expect(radix(36)).toBe(1);
  expect(radix(0)).toBe(2);
  expect(radix(7)).toBe(3);
});

// J: `const LATE = EARLY + 1;` written above `const EARLY = 1;` is
// `ReferenceError: Cannot access 'EARLY' before initialization` at module load,
// so the whole file failed to load. That this test runs at all is half the
// point; the value is the other half.
test('a const whose initialiser names a later one loads and answers', () => {
  expect(ordered()).toBe(3);
});

// K: Rust's atomics WRAP at their width whatever the build's debug assertions
// say. `+= 1` on a `number` went on counting, so the port answered 4294967296
// where Rust answers 0.
test('an atomic wraps at its width', () => {
  expect(wrapAround()).toBe(4294967295);
  expect(wrapAround()).toBe(0);
});

// D5: a constant Rust puts on a primitive type. At the parent this line read
// `f64.EPSILON.max(..)` — an undeclared name AND a method a JavaScript number
// has not got — and no diagnostic said so.
test('a primitive`s associated constant is the constant, and types the call on it', () => {
  expect(epsilonNear(0)).toBe(Number.EPSILON);
  // `Math.abs(1e9) * EPSILON` is the larger of the two, which is what Rust
  // picks here.
  expect(epsilonNear(1e9)).toBe(Math.abs(1e9) * Number.EPSILON);
});

test('and the width constants are the numbers those widths hold', () => {
  const [maxU32, minI64, maxU64, inf, nan] = widths();
  expect(maxU32).toBe(4294967295);
  expect(minI64).toBe(-9223372036854775808n);
  expect(maxU64).toBe(18446744073709551615n);
  expect(inf).toBe(Infinity);
  expect(Number.isNaN(nan)).toBe(true);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
