// Runs the emitted lift_without_a_transfer against the real runtime. What is
// under test is who owns a LIFTED argument when the call it was lifted for
// never takes it.
//
// Three defective paths are driven. `refusedCallee` and `refusedCalleeUnflagged`
// reach a hole where the port refused the call the clone was lifted for: the
// parent's engine set the lift's flag immediately above that hole (and, in the
// unflagged arm, wrote no release at all), so the clone was released by nobody
// and the collector reported it. `make` on a released handle throws out of
// `this.deref().value.n`, which the parent evaluated BELOW the move flag, so
// the token the constructor never took was reported handed over.

import { expect, test } from 'bun:test';
import { dropOwned } from '@ankurah/base';
import { Handle, Inner, Rows, Spill, Token, refusedCallee, refusedCalleeUnflagged } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the clone lifted for a refused call is released when the hole throws', () => {
  const spill = new Spill(1);
  expect(() => refusedCallee(new Rows(2), spill, 3, false)).toThrow('collect');
  // `spill` itself is the caller's and the callee released it on the way out;
  // what the parent leaked is the CLONE the port lifted for `top_k`.
  expect(spill.isDropped).toBe(true);
});

test('the same call hands back zero when it leaves before the hole', () => {
  const rows = new Rows(2);
  expect(refusedCallee(rows, new Spill(1), 3, true)).toBe(0);
  expect(rows.isDropped).toBe(true);
});

test('an unflagged lift above a hole is released too', () => {
  const spill = new Spill(1);
  expect(() => refusedCalleeUnflagged([new Token(1)], spill, 3, false)).toThrow('collect');
  expect(spill.isDropped).toBe(true);
});

test('make hands back the inner number when the handle carries one', () => {
  const handle = new Handle(new Inner(7));
  const token = new Token(1);
  expect(handle.make(token, false)).toBe(7);
  dropOwned(handle);
});

test('the token is released when the deref above the flag throws', () => {
  const handle = new Handle(null);
  const token = new Token(1);
  // `this.deref()` is where the throw stands, and the parent had already
  // written `_moved0 = true` above it, so the `finally` believed the
  // constructor had taken the token.
  expect(() => handle.make(token, false)).toThrow('unwrap');
  expect(token.isDropped).toBe(true);
  dropOwned(handle);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
