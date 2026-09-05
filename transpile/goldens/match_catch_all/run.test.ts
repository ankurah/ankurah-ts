// Runs the emitted match_catch_all against the real runtime. What is under test
// is the arm that names no variant: the runtime's match dispatches on the
// variant name, so an arm with no name to be written under used to vanish, and
// the enum's non-exhaustive fatal fired for every value the other arms did not
// name. Each test below takes that arm.

import { expect, test } from 'bun:test';
import { Cause, Held, Inner, Order, Reason, Wrapped, asArgument, count, ignore, ignoreNamed, letInit, lift, rank, refutable, sameName, tally, tieBreak, unwind, widen } from './input.ts';
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

test('the value position gets the arm\'s value, not undefined', () => {
  // The expanded arms used to be written by hand rather than through the arm
  // renderer, and outside the enclosing function\'s return position they had no
  // `return` at all: the local was `undefined` for every value the named arms
  // did not cover.
  const other = new Cause('Other', {});
  expect(letInit(other)).toBe(3);
  expect(asArgument(other)).toBe(6);
  other.drop();
  const denied = new Cause('Denied', { _0: new Inner(10) });
  expect(letInit(denied)).toBe(11);
  expect(asArgument(denied)).toBe(20);
  denied.drop();
});

test('a consuming catch-all releases the payload it reads nothing of', () => {
  // `intoMatch` hands the whole payload over and keeps none of it, so this arm
  // is the only thing that can release the Inner inside a Second.
  expect(ignore(new Held('Second', { _0: new Inner(4) }))).toBe(0);
  expect(ignore(new Held('First', { _0: new Inner(7) }))).toBe(7);
});

test('a named arm that ignores its payload releases it too', () => {
  expect(ignoreNamed(new Held('First', { _0: new Inner(1) }))).toBe(1);
  expect(ignoreNamed(new Held('Second', { _0: new Inner(2) }))).toBe(2);
  expect(ignoreNamed(new Held('Third', { _0: new Inner(3) }))).toBe(3);
});

test('an arm that tests inside its variant does not delete the catch-all', () => {
  const missing = new Reason('Cause', { _0: new Cause('Missing', {}) });
  expect(refutable(missing)).toBe(5);
  missing.drop();
  // The value the testing arm does not match: it used to reach an arm written
  // for `Cause` anyway, because the arm was counted as covering the variant.
  const other = new Reason('Cause', { _0: new Cause('Other', {}) });
  expect(refutable(other)).toBe(6);
  other.drop();
  const plain = new Reason('Plain', {});
  expect(refutable(plain)).toBe(6);
  plain.drop();
});

test('a catch-all that shadows the scrutinee binds it once', () => {
  const missing = sameName(new Cause('Missing', {}));
  expect(missing.type).toBe('Missing');
  missing.drop();
  const denied = sameName(new Cause('Denied', { _0: new Inner(5) }));
  expect(denied.type).toBe('Denied');
  denied.drop();
});

test('an unwind out of a consuming arm has one owner', () => {
  // The arm binds `inner`, throws, and releases `inner` in its `finally`.
  // `intoMatch` used to release the payload as the exception went past, so what
  // came out was `BUG: Inner was dropped twice` rather than the panic.
  expect(() => unwind(new Cause('Denied', { _0: new Inner(6) })))
    .toThrow('width 6 is not allowed');
  expect(unwind(new Cause('Other', {}))).toBe(0);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
