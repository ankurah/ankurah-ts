// Runs the emitted opaque_iterator against the real runtime. What is under test
// is that an iterator a generic body advances by hand is a CURSOR: `next()`
// hands out one element and leaves the rest, and what the walk never reached is
// released when the iterator goes out of scope.
//
// The defective path is a walk that stops early — `sumFirst(tokens, 1)` over
// three tokens — where two elements are still the iterator's. Against the
// parent's engine `next` is a hole and every one of these throws.

import { expect, test } from 'bun:test';
import { SeqCursor } from '@ankurah/base';
import { Refused, Token, restOf, sumFirst, takeSome } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a walk that reaches every element hands back the sum', () => {
  expect(sumFirst([new Token(1), new Token(2)], 2).unwrap()).toBe(3);
});

test('what the walk never reached is released with the iterator', () => {
  // Two of the three tokens are still the cursor's when `sumFirst` leaves, and
  // the `Err` it answers is the only thing the caller receives.
  const failed = sumFirst([new Token(1), new Token(2), new Token(3)], 1);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
});

test('a walk that asks for more than there is answers Err and releases nothing twice', () => {
  const failed = sumFirst([new Token(4)], 2);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
});

test('an empty sequence walks to nothing', () => {
  expect(sumFirst([], 0).unwrap()).toBe(0);
});

test('takeSome on its own advances the cursor it is handed', () => {
  const walk = new SeqCursor([new Token(5), new Token(6)]);
  try {
    expect(takeSome(walk, 1).unwrap()).toBe(5);
    // The second token is still the cursor's, and its drop releases it.
  } finally {
    walk.drop();
  }
  const empty = new SeqCursor<Token>([]);
  try {
    const refused = takeSome(empty, 1).unwrapErr();
    expect(refused).toBeInstanceOf(Refused);
    refused.drop();
  } finally {
    empty.drop();
  }
});

// A cursor asked for anything but `next` gives up its rest: the loop below
// consumes what the walk had not reached, and the cursor is left holding
// nothing. Written without it the loop iterated a `SeqCursor`, which has no
// `Symbol.iterator` at all.
test('the rest of a part-walked cursor is what the loop sees', () => {
  const kept = restOf([new Token(1), new Token(2), new Token(3)], 1).unwrap();
  expect(kept.map((t) => t.n)).toEqual([2, 3]);
  for (const token of kept) token.drop();
});

test('a cursor the walk exhausted has no rest', () => {
  const kept = restOf([new Token(7)], 1).unwrap();
  expect(kept.length).toBe(0);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
