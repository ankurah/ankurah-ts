// Runs the emitted value_equality against the real runtime. At the parent
// `isZero` and `sameMembers` were `===` between two objects — identity — so
// both answered false for every pair Rust calls equal.

import { expect, test } from 'bun:test';
import { HashSet } from '@ankurah/base';
import { differentTag, isZero, Kind, sameKind, sameMembers, Tag } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('two byte buffers compare by content', () => {
  expect(isZero(new Uint8Array([0, 0, 0, 0]))).toBe(true);
  expect(isZero(new Uint8Array([0, 1, 0, 0]))).toBe(false);
  // Not the same length: Rust's `==` is false, and so is this.
  expect(isZero(new Uint8Array([0, 0]))).toBe(false);
});

test('two sets compare by membership, whatever order they were filled in', () => {
  const a = HashSet.from([1, 2, 3]);
  const b = HashSet.from([3, 1, 2]);
  expect(sameMembers(a, b)).toBe(true);
  b.add(4);
  expect(sameMembers(a, b)).toBe(false);
  a.drop();
  b.drop();
});

test('two freshly built enum values compare by variant', () => {
  // `Kind` is `Copy` in Rust and an `Enum` object here, so the driver drops
  // what it built: the comparison takes neither operand.
  const small = new Kind('Small', {});
  const alsoSmall = new Kind('Small', {});
  const large = new Kind('Large', {});
  expect(sameKind(small, alsoSmall)).toBe(true);
  expect(sameKind(small, large)).toBe(false);
  for (const k of [small, alsoSmall, large]) k.drop();
});

test('a string field keeps the JavaScript operator', () => {
  const a = new Tag('x');
  const b = new Tag('y');
  expect(differentTag(a, b)).toBe(true);
  const sameName = new Tag('x');
  expect(differentTag(a, sameName)).toBe(false);
  for (const t of [a, b, sameName]) t.drop();
});

test('nothing leaked', async () => {
  await expectNoOwnershipReports();
});
