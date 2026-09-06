// Runs the emitted taken_by_value against the real runtime. Against the parent
// engine (c2e2b2d) every closure here leaks what the call handed it, and the
// comparison arms leak both operands: a by-value parameter and an arm's binding
// were owned by nobody.

import { expect, test } from 'bun:test';
import { OwnershipFatal, clearFatalLatch } from '@ankurah/base';
import {
  Holder,
  Token,
  findBorrowed,
  firstKept,
  items,
  keepFirst,
  positionOf,
  positionOrFail,
  total,
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

test('a by-value closure parameter is released when the invocation returns', () => {
  const { tokens, dropped } = three();
  // The defective answer: `[1]` — only the element the walk never reached was
  // released, and the two the callback was handed leaked.
  expect(positionOf(tokens, 2)).toBe(1);
  expect(dropped()).toEqual([1, 2, 3]);
});

test('a by-value closure parameter is released when the invocation throws', () => {
  const { tokens, dropped } = three();
  expect(() => positionOrFail(tokens, 2)).toThrow('bad token');
  // 1 went through the callback and came back; 2 was in the callback when it
  // threw; 3 was never reached and the terminal released it.
  expect(dropped()).toEqual([1, 2, 3]);
  clearFatalLatch();
});

test('a parameter the closure hands on is released by nothing here', () => {
  const { tokens, dropped } = three();
  const got = firstKept(tokens);
  expect(got!.isDropped).toBe(false);
  expect(got!._0).toBe(1);
  expect(dropped()).toEqual([2, 3]);
  got!.drop();
});

test('a field read in a closure’s expression body takes the field out', () => {
  const holders = [1, 2].map((n) => new Holder(new Token(n), n));
  const got = items(holders);
  expect(got.map((t) => t._0)).toEqual([1, 2]);
  // The holders are gone and the tokens they held are the caller's.
  for (const h of holders) expect(h.isDropped).toBe(true);
  for (const t of got) expect(t.isDropped).toBe(false);
  for (const t of got) t.drop();
});

test('a borrowed chain releases nothing', () => {
  const { tokens, dropped } = three();
  expect(findBorrowed(tokens, 2)?._0).toBe(2);
  expect(dropped()).toEqual([]);
  for (const t of tokens) t.drop();
});

test('both operands an arm bound are released where the arm ends', () => {
  const x = new Token(1);
  const y = new Token(2);
  // The defective answer: 3, and both operands still alive, owned by nobody —
  // neither binding is the whole subject, so neither was claimed.
  expect(total(x, y)).toBe(3);
  expect(x.isDropped).toBe(true);
  expect(y.isDropped).toBe(true);
});

test('an arm that hands its binding on releases it nowhere', () => {
  const x = new Token(3);
  const y = new Token(4);
  const got = keepFirst(x, y);
  expect(got!.isDropped).toBe(false);
  expect(got!._0).toBe(3);
  got!.drop();
  // Position 1 is the one this arm wrote `_` for, and the arm owes it.
  expect(y.isDropped).toBe(true);
});

test('nothing leaked and nothing was dropped twice', async () => {
  expect(OwnershipFatal).toBeDefined();
  await expectNoOwnershipReports();
});
