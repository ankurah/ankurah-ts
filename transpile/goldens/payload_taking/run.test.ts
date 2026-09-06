// Runs the emitted payload_taking against the real runtime. Each test below is
// a runtime answer the parent engine (c723a60) got wrong — a leaked wrapper, a
// moved `Result` the caller still owned, a tuple released twice.

import { expect, test } from 'bun:test';
import { Result, UnsupportedShape } from '@ankurah/base';
import { Count, Holder, Inner, Outer, Token, both, consumed, counted, either, inside } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an arm that takes a droppable name out of a member refuses, and releases what it holds', () => {
  // The parent bound `t`, dropped it, and left the `Inner` that held it to
  // nobody: `dropUnbound` excluded `_0` because a name had taken something out
  // of it, and nothing else released the wrapper.
  const w = new Outer('W', { _0: new Inner('X', { _0: Token.new(5) }) });
  expect(() => inside(w)).toThrow(UnsupportedShape);
});

test('and the arm below the refusing one still runs', () => {
  // The refusal stands in the BRANCH, so the test still decides.
  expect(inside(new Outer('W', { _0: new Inner('Y', { _0: Token.new(1) }) }))).toBe(1);
  expect(inside(new Outer('Z', {}))).toBe(0);
});

test('the same question through an `|` is answered the same way', () => {
  const x = new Outer('W', { _0: new Inner('X', { _0: Token.new(2) }) });
  expect(() => either(x)).toThrow(UnsupportedShape);
  const y = new Outer('W', { _0: new Inner('Y', { _0: Token.new(3) }) });
  expect(() => either(y)).toThrow(UnsupportedShape);
  expect(either(new Outer('Z', {}))).toBe(0);
});

test('a member the pattern reaches inside without taking anything droppable is released', () => {
  // The parent read `n` out of the `Count` and released neither the `Count` nor
  // the `Holder` payload around it: an `|` was read as touching nothing, so the
  // member went into the "somebody took this" list and out of the release.
  expect(counted(new Holder('Held', { _0: new Count('Small', { _0: 4 }) }))).toBe(4);
  expect(counted(new Holder('Held', { _0: new Count('Large', { _0: 9 }) }))).toBe(9);
  expect(counted(new Holder('Empty', {}))).toBe(0);
});

test('a tuple of borrowed Results is not taken apart', () => {
  const left = Result.Ok(Token.new(2));
  const right = Result.Ok(Token.new(3));
  expect(both(left, right)).toBe(5);
  // The parent called `unwrap()` on each, which marks the `Result` moved: this
  // second read was `Result was used after being moved`.
  expect(both(left, right)).toBe(5);
  left.unwrap().drop();
  right.unwrap().drop();
});

test('a consuming tuple pattern releases its elements once', () => {
  // The parent released `a` and `b` in the arm AND the tuple around them in a
  // `finally`, which is a double drop the runtime reports.
  expect(consumed([Token.new(1), Token.new(2)])).toBe(3);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
