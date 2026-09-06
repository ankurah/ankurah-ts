// Runs the emitted owned_adaptors against the real runtime. Against the parent
// engine (c2e2b2d) every eager adaptor here forgets what it discarded, the
// reading key fold builds a key per element and releases none of them, and
// `next` on a fresh receiver is a hole.

import { expect, test } from 'bun:test';
import { OwnershipFatal } from '@ankurah/base';
import {
  Key,
  Token,
  borrowedFilter,
  everyOther,
  firstBorrowed,
  firstOver,
  firstOwned,
  middle,
  widest,
  widestOwned,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

/** Four tokens, and a record of which numbers have been released. */
function four(): { tokens: Token[]; dropped: () => number[] } {
  const seen: number[] = [];
  const tokens = [0, 1, 2, 3].map((n) => {
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

test('filter releases what its predicate rejected', () => {
  const { tokens, dropped } = four();
  // Token 0 fails the filter; 1 and 3 are walked past by `find`; 2 is answered.
  const got = firstOver(tokens, 2);
  expect(got?._0).toBe(2);
  // The defective answer: `[1, 3]` — token 0 was erased by `Array.filter` and
  // the terminal below never saw it.
  expect(dropped()).toEqual([0, 1, 3]);
  got!.drop();
});

test('step_by releases what it stepped over', () => {
  const { tokens, dropped } = four();
  const got = everyOther(tokens);
  expect(got?._0).toBe(2);
  // 1 and 3 were stepped over; 0 was walked past by `last`.
  expect(dropped()).toEqual([0, 1, 3]);
  got!.drop();
});

test('skip releases the prefix and take releases the tail', () => {
  const { tokens, dropped } = four();
  const got = middle(tokens);
  expect(got?._0).toBe(1);
  expect(dropped()).toEqual([0, 2, 3]);
  got!.drop();
});

test('a borrowed chain discards nothing', () => {
  const { tokens, dropped } = four();
  expect(borrowedFilter(tokens, 2)?._0).toBe(2);
  expect(dropped()).toEqual([]);
  for (const t of tokens) t.drop();
});

/** How many `Key`s have been released, counted through the prototype. */
let keysDropped = 0;
const keyDrop = (Key.prototype as unknown as { drop: () => void }).drop;
(Key.prototype as unknown as { drop: () => void }).drop = function (this: Key) {
  keysDropped += 1;
  return keyDrop.call(this);
};

test('the reading key fold releases every key it built', () => {
  const { tokens } = four();
  keysDropped = 0;
  const got = widest(tokens);
  expect(got?._0).toBe(3);
  // Rust builds one key per element as `map(|x| (f(&x), x))`, drops the loser's
  // pair — key and all — and destructures the winner's. The defective answer:
  // 0, every key owned by nobody.
  expect(keysDropped).toBe(4);
  // The elements are borrows and are still the caller's.
  for (const t of tokens) expect(t.isDropped).toBe(false);
  for (const t of tokens) t.drop();
});

test('the consuming key fold releases the losers as well as their keys', () => {
  const { tokens, dropped } = four();
  keysDropped = 0;
  const got = widestOwned(tokens);
  expect(got?._0).toBe(3);
  expect(dropped()).toEqual([0, 1, 2]);
  expect(keysDropped).toBe(4);
  got!.drop();
});

test('next on a receiver nobody else holds answers the head and drops the tail', () => {
  const { tokens, dropped } = four();
  const got = firstOwned(tokens);
  expect(got?._0).toBe(0);
  // The defective answer: a hole — `next` was refused whatever the receiver.
  expect(dropped()).toEqual([1, 2, 3]);
  got!.drop();
});

test('and on a borrowed chain it reads through', () => {
  const { tokens, dropped } = four();
  expect(firstBorrowed(tokens)?._0).toBe(0);
  expect(dropped()).toEqual([]);
  for (const t of tokens) t.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  expect(OwnershipFatal).toBeDefined();
  await expectNoOwnershipReports();
});
