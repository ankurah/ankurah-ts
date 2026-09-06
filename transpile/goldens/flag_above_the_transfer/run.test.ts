// Runs the emitted flag_above_the_transfer against the real runtime. What is
// under test is WHERE a move flag stands: a flag says "somebody else owns this
// now", so it must not be set until the value has actually been handed over.
//
// Three defective paths are driven. `lifted(true)` throws out of the argument
// the port lifted above the flag; `twoTransfers(true)` leaves through the FIRST
// `?` with the second token still in hand; `oneTransfer(true)` leaves through
// the `?` of the very call that took the token, which the callee has already
// released. The first two leak against the parent's engine, and the third is
// the double drop that keeps the flag above its own hoist.

import { expect, test } from 'bun:test';
import { Refused, Token, build, consume, eat, gate, lifted, oneTransfer, twoTransfers } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('lifted hands back the sum when nothing throws', () => {
  expect(lifted(false).unwrap()).toBe(6);
});

test('the held token is released when the lifted argument throws', () => {
  // `build` throws where the flag used to already be set, so `held` was handed
  // to nobody and reported as handed over.
  expect(() => lifted(true)).toThrow('build exploded');
});

test('twoTransfers hands back the sum when neither ? leaves', () => {
  expect(twoTransfers(false).unwrap()).toBe(4);
});

test('the second token is released when the first ? returns Err', () => {
  const failed = twoTransfers(true);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
});

test('oneTransfer hands back the sum when the ? does not leave', () => {
  expect(oneTransfer(false).unwrap()).toBe(4);
});

test('the token the callee released is not released again', () => {
  // The flag stands above the hoist here: the callee took the token and
  // dropped it on its own early return, and the block must believe that.
  const failed = oneTransfer(true);
  expect(failed.isErr()).toBe(true);
  failed.unwrapErr().drop();
});

test('the pieces the golden is built from work on their own', () => {
  expect(gate(true).unwrap()).toBe(1);
  const closed = gate(false);
  expect(closed.isErr()).toBe(true);
  closed.unwrapErr().drop();
  const token = build(false);
  expect(token).toBeInstanceOf(Token);
  expect(eat(token, new Token(5)).unwrap()).toBe(9);
  const refused = consume(new Token(7), true).unwrapErr();
  expect(refused).toBeInstanceOf(Refused);
  refused.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
