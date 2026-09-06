// Runs the emitted owned_terminals against the real runtime. F1: against the
// parent engine (b05f82c) every consuming terminal here is wrong — `position`
// raises `OwnershipFatal` because the sequence is released a second time,
// `find` hands back an element the same `finally` has already released, and
// `max_by_key`, `min_by` and `reduce` leak every element they did not answer.

import { expect, test } from 'bun:test';
import { OwnershipFatal, clearFatalLatch } from '@ankurah/base';
import {
  Token,
  biggest,
  borrowedFind,
  findOne,
  firstEven,
  firstKept,
  lastOf,
  peekLast,
  positionOf,
  smallest,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

/** Three tokens, and a record of which numbers have been released. */
function three(): { tokens: Token[]; dropped: () => number[] } {
  const seen: number[] = [];
  const tokens = [1, 2, 3].map((n) => {
    const t = new Token(n);
    const inner = t.drop.bind(t);
    t.drop = () => {
      if (!t.isDropped) seen.push(n);
      inner();
    };
    return t;
  });
  return { tokens, dropped: () => [...seen].sort((a, b) => a - b) };
}

test('a closure that drops the element it was given does not drop it twice', () => {
  const { tokens, dropped } = three();
  // The defective answer: `OwnershipFatal`, because the emitted `finally`
  // released the whole sequence after the closure had released each element.
  expect(positionOf(tokens, 2)).toBe(1);
  // 1 and 2 went through the closure, which dropped them; 3 was never reached
  // and the terminal released it.
  expect(dropped()).toEqual([1, 2, 3]);
  clearFatalLatch();
});

test('the element a consuming find answers is the caller’s, and the rest are gone', () => {
  const { tokens, dropped } = three();
  const found = findOne(tokens, 2);
  expect(found).not.toBe(null);
  // The defective answer: `found.isDropped` is true — the `finally` released
  // the sequence the element came out of.
  expect(found!.isDropped).toBe(false);
  expect(dropped()).toEqual([1, 3]);
  found!.drop();
});

test('max_by_key, min_by, reduce and last release every element they do not answer', () => {
  for (const [what, call] of [
    ['max_by_key', (ts: Token[]) => biggest(ts)],
    ['min_by', (ts: Token[]) => smallest(ts)],
    ['reduce', (ts: Token[]) => firstKept(ts)],
    ['last', (ts: Token[]) => lastOf(ts)],
  ] as const) {
    const { tokens, dropped } = three();
    const got = call(tokens);
    expect(got, what).not.toBe(null);
    expect(got!.isDropped, what).toBe(false);
    // The defective answer: nothing at all was released.
    expect(dropped().length, what).toBe(2);
    got!.drop();
    expect(dropped().length, what).toBe(3);
  }
});

test('a borrowed chain releases nothing, and the sequence stays the caller’s', () => {
  const { tokens, dropped } = three();
  expect(borrowedFind(tokens, 2)?._0).toBe(2);
  expect(peekLast(tokens)?._0).toBe(3);
  expect(dropped()).toEqual([]);
  for (const t of tokens) t.drop();
});

test('elements with no drop glue are read exactly as they were', () => {
  expect(firstEven([1, 3, 4, 6])).toBe(4);
  expect(firstEven([1, 3])).toBe(null);
});

test('nothing leaked and nothing was dropped twice', async () => {
  expect(OwnershipFatal).toBeDefined();
  await expectNoOwnershipReports();
});
