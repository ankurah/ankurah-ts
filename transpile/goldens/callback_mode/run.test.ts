// Runs the emitted callback_mode against the real runtime. Against the parent
// engine (c2e2b2d) the borrowed callbacks are released by the terminal, so
// `findBorrowing`'s own `p.drop()` is a second drop and `readBorrowing`'s
// second `find` calls a closure whose captures are gone; and `throughByRef`
// emits `it.byRef()`, a method no array declares.

import { expect, test } from 'bun:test';
import { OwnershipFatal, clearFatalLatch } from '@ankurah/base';
import {
  Token,
  borrowedThroughByRef,
  findBorrowing,
  findOwning,
  readBorrowing,
  throughByRef,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

function three(): Token[] {
  return [1, 2, 3].map((n) => new Token(n));
}

test('a callback taken by value is released by the terminal, captures and all', () => {
  const want = new Token(2);
  const found = findOwning(three(), want);
  expect(found?._0).toBe(2);
  // The closure went with the call, and dropping it dropped what it captured.
  expect(want.isDropped).toBe(true);
  found!.drop();
});

test('a callback taken by &mut is the caller’s, and is released once', () => {
  const want = new Token(2);
  const found = findBorrowing(three(), want);
  expect(found?._0).toBe(2);
  // The defective answer: `OwnershipFatal` — the terminal released the closure
  // and the block's own `drop(p)` released it again.
  expect(want.isDropped).toBe(true);
  found!.drop();
});

test('a callback taken by & is called again after the terminal', () => {
  const want = new Token(2);
  // `read_borrowing` BORROWS the sequence, so the tokens stay the driver's.
  const tokens = three();
  // The defective answer: the second `find` calls a closure the first one
  // released, and reads a capture that is gone.
  expect(readBorrowing(tokens, want)).toBe(2);
  expect(want.isDropped).toBe(true);
  for (const t of tokens) t.drop();
});

test('a consuming terminal reached through by_ref is refused, and the block keeps the iterator', () => {
  const tokens = three();
  // The defective answer: `TypeError: it.byRef is not a function`.
  expect(() => throughByRef(tokens)).toThrow(/consumes the elements it walks/);
  for (const t of tokens) expect(t.isDropped).toBe(true);
  clearFatalLatch();
});

test('by_ref on a borrowed chain is the identity', () => {
  const tokens = three();
  expect(borrowedThroughByRef(tokens)?._0).toBe(1);
  for (const t of tokens) expect(t.isDropped).toBe(false);
  for (const t of tokens) t.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  expect(OwnershipFatal).toBeDefined();
  await expectNoOwnershipReports();
});
