// Runs the emitted byte_targets against the real runtime. E1: the parent engine
// (b05f82c) answered a `number[]` from every function here whose target is a
// `Vec<u8>`, so `instanceof Uint8Array` was false and every byte reader
// downstream — `encode`, a `TextDecoder`, a comparison by byte — was reading
// the wrong kind of object.

import { expect, test } from 'bun:test';
import { copyOf, descending, descendingLocal, doubled, firstComplement, oneByte } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a descending index component is BYTES, complemented', () => {
  const out = descending(new Uint8Array([0, 1, 254, 255]));
  // The defective answer: a plain array, which passes neither of these.
  expect(out).toBeInstanceOf(Uint8Array);
  expect(Array.isArray(out)).toBe(false);
  expect([...out]).toEqual([255, 254, 1, 0]);
});

test('and the same through a `let` annotation and a turbofish', () => {
  expect(descendingLocal(new Uint8Array([1, 2, 3]))).toBe(3);
  expect(firstComplement(new Uint8Array([1, 2, 3]))).toBe(254);
});

test('the `vec![..]` spelling of the same answer agrees with it', () => {
  const one = oneByte(1);
  expect(one).toBeInstanceOf(Uint8Array);
  expect([...one]).toEqual([254]);
  // Both spellings of "the complement of one byte" build the same thing.
  expect([...descending(new Uint8Array([1]))]).toEqual([...one]);
});

test('a `Vec` of anything else is still an array', () => {
  const ns = doubled([1, 2, 3]);
  expect(Array.isArray(ns)).toBe(true);
  expect(ns).toEqual([2, 4, 6]);
});

test('bytes collected out of a borrowed slice are bytes too', () => {
  const src = new Uint8Array([7, 8, 9]);
  const copy = copyOf(src);
  expect(copy).toBeInstanceOf(Uint8Array);
  expect([...copy]).toEqual([7, 8, 9]);
  // `collect` builds a NEW collection: writing to the copy leaves the source.
  copy[0] = 0;
  expect(src[0]).toBe(7);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
