// Runs the emitted cursor_boundary against the real runtime.
//
// Three claims, each of which the parent's engine got wrong on its own. A
// concrete caller's array is WRAPPED where it crosses into a cursor parameter —
// handed over as it stood, every `walk.next()` and `walk.takeRest()` inside the
// callee threw. A by-value cursor parameter is RELEASED by the body that owns
// it, so what the walk never reached goes with it — read off the unmodified
// Rust type the parameter owned nothing and the whole sequence leaked. And a
// cursor that gave up its rest is dispatched as a SEQUENCE, so `last` and
// `skip` reach the owned helpers that release what they walk past.

import { expect, test } from 'bun:test';
import { SeqCursor } from '@ankurah/base';
import { Token, allBut, dropped, head, headOf, tail } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a concrete caller hands a cursor over, and the callee keeps one element', () => {
  const first = head([new Token(1n), new Token(2n), new Token(3n)]);
  expect(first!.n).toBe(1n);
  // Tokens 2 and 3 were still the cursor's, and `first`'s own drop released
  // them.
  first!.drop();
});

test('a cursor parameter the body never advances releases the whole sequence', () => {
  expect(dropped([new Token(1n), new Token(2n)])).toBe(7n);
});

test('an owned terminal on the rest releases everything it walked past', () => {
  const last = tail([new Token(1n), new Token(2n), new Token(3n)]);
  expect(last!.n).toBe(3n);
  last!.drop();
});

test('an owned adaptor on the rest releases the prefix it skipped', () => {
  const kept = allBut([new Token(1n), new Token(2n), new Token(3n)], 1);
  expect(kept.map((t) => t.n)).toEqual([2n, 3n]);
  for (const token of kept) token.drop();
});

test('a caller that already holds a cursor hands it over as it stands', () => {
  const first = headOf(new SeqCursor([new Token(8n), new Token(9n)]));
  expect(first!.n).toBe(8n);
  first!.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
