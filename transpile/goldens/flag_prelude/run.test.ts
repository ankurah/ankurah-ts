// Runs the emitted flag_prelude against the real runtime. Against the parent
// engine (c2e2b2d) every function here leaks its Token on the throwing path —
// the flag said the callee had taken it and the callee was never reached — and
// `insideABranch` is worse than that: it wrote no flag at all, so the callee
// took the token and the block released it a second time.

import { expect, test } from 'bun:test';
import { OwnershipFatal, clearFatalLatch } from '@ankurah/base';
import {
  Sink,
  Token,
  fieldOfCall,
  indexOfCall,
  insideABranch,
  throughTry,
  throwingReceiver,
  twoLifts,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

/** A token that says whether it has been released. */
function token(n = 1): Token {
  return new Token(n);
}

test('a field of a call: the flag stands below the call', () => {
  const c = token();
  expect(() => fieldOfCall(c, true, false)).toThrow('boom');
  // The defective answer: `c.isDropped` is false and nothing owns it — the
  // flag was set before `boom()` ran.
  expect(c.isDropped).toBe(true);
  clearFatalLatch();

  const kept = token();
  expect(fieldOfCall(kept, false, false)).toBe(1);
  expect(kept.isDropped).toBe(true);
});

test('an index that is a call: the same', () => {
  const c = token();
  expect(() => indexOfCall(c, [7], true, false)).toThrow('boom');
  expect(c.isDropped).toBe(true);
  clearFatalLatch();
});

test('a receiver that throws: the flag stands below it', () => {
  const c = token();
  expect(() => throwingReceiver(null, c, false)).toThrow(/Option::unwrap/);
  expect(c.isDropped).toBe(true);
  clearFatalLatch();

  const kept = token();
  const sink = new Sink();
  expect(throwingReceiver(sink, kept, false)).toBe(1);
  expect(kept.isDropped).toBe(true);
});

test('inside a branch: the flag is written at all, and below the lift', () => {
  const c = token();
  // The defective answer: `OwnershipFatal` — the arm wrote no flag, so `eat`
  // released the token and the block's `finally` released it again.
  expect(insideABranch(c, 4, false)).toBe(5);
  expect(c.isDropped).toBe(true);
  // And the path the arm does not take leaves the token to the block.
  const other = token();
  expect(insideABranch(other, null, false)).toBe(0);
  expect(other.isDropped).toBe(true);
});

test('two lifts: the first is released when the second throws', () => {
  const c = token();
  expect(() => twoLifts(c, null, false)).toThrow(/Option::unwrap/);
  expect(c.isDropped).toBe(true);
  clearFatalLatch();

  const kept = token();
  expect(twoLifts(kept, 3, false)).toBe(3);
  expect(kept.isDropped).toBe(true);
});

test('a `?` lifts the call that consumes, so its flag stands above the lift', () => {
  const c = token(5);
  const err = throughTry(c, true, false);
  expect(err.isErr()).toBe(true);
  err.drop();
  // `give` took the token and released it; the block must not release it again.
  expect(c.isDropped).toBe(true);

  const ok = token(6);
  const got = throughTry(ok, false, false);
  expect(got.unwrap()).toBe(6);
  expect(ok.isDropped).toBe(true);
});

test('nothing leaked and nothing was dropped twice', async () => {
  expect(OwnershipFatal).toBeDefined();
  await expectNoOwnershipReports();
});
