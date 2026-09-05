// Runs the emitted consts_and_literals against the real runtime. Two claims:
// a module-level `const` and a `static` carry their VALUE, and a struct literal
// hands each value to the field it was written beside.

import { expect, test } from 'bun:test';
import { FLOOR, Rec, SYSTEM_COLLECTION, TAG_NULL, TAG_STRING, WORDS, arm, bump, collection, movedOrigin, radix, shifted, word } from './input.ts';
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

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
