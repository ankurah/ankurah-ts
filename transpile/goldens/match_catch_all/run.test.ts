// Runs the emitted match_catch_all against the real runtime. What is under test
// is the arm that names no variant: the runtime's match dispatches on the
// variant name, so an arm with no name to be written under used to vanish, and
// the enum's non-exhaustive fatal fired for every value the other arms did not
// name. Each test below takes that arm.

import { expect, test } from 'bun:test';
import { Cause, Inner, Order, Wrapped, count, lift, rank, tally, tieBreak, widen } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a named catch-all hands back the value it was given', () => {
  // The path that used to be fatal: everything that is not the tie.
  const picked = tieBreak(new Order('Greater', {}), new Order('Less', {}));
  expect(picked.type).toBe('Greater');
  picked.drop();
});

test('the tie takes the named arm and the value it did not use is released', () => {
  const picked = tieBreak(new Order('Equal', {}), new Order('Less', {}));
  expect(picked.type).toBe('Less');
  picked.drop();
});

test('a wildcard arm reads the subject the tested arms did not take', () => {
  const wrapped = lift(new Cause('Missing', {}));
  expect(wrapped.type).toBe('Whole');
  wrapped.drop();
});

test('the tested arm of the same match still takes the payload out', () => {
  const wrapped = lift(new Cause('Denied', { _0: new Inner(7) }));
  expect(wrapped.type).toBe('Held');
  wrapped.drop();
});

test('several named arms make one test between them', () => {
  const denied = new Cause('Denied', { _0: new Inner(4) });
  expect(rank(denied)).toBe(4);
  denied.drop();
  const missing = new Cause('Missing', {});
  expect(rank(missing)).toBe(1);
  missing.drop();
  const other = new Cause('Other', {});
  expect(rank(other)).toBe(0);
  other.drop();
});

test('a catch-all in statement position runs for its effect', () => {
  const into: number[] = [];
  const denied = new Cause('Denied', { _0: new Inner(3) });
  widen(denied, into);
  denied.drop();
  const other = new Cause('Other', {});
  widen(other, into);
  other.drop();
  expect(into).toEqual([3, 0]);
});

test('a named catch-all that only reads its value releases it itself', () => {
  // Nothing is returned, so the only thing that can release the Cause is the
  // arm that bound it — a leak here is the transpiler's.
  expect(tally(new Cause('Other', {}))).toBe(1);
  expect(tally(new Cause('Denied', { _0: new Inner(9) }))).toBe(9);
});

test('count borrows and leaves the Cause to its owner', () => {
  const cause = new Cause('Missing', {});
  expect(count(cause)).toBe(1);
  expect(count(cause)).toBe(1);
  cause.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
