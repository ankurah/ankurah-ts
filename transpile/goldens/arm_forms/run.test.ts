// Runs the emitted arm_forms against the real runtime. Each test below is an
// answer the parent engine (c723a60) got wrong, and every one of the three
// wrong answers came from reading rendered TypeScript back instead of carrying
// what the lowering wrote (K2).

import { expect, test } from 'bun:test';
import { Answer, Holder, Source, Token, Weight, pick, record, resolve, resolveTwice, tally, weigh } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a nested match IS the arm value, so the arm hands it back', () => {
  // The defective answer: `Absent` fell out of the arrow with no `return`, and
  // the caller read `undefined`. This is `ankql::ast::Expr::populate_recursive`.
  const found = resolve(new Source('Absent', {}), 7);
  expect(found).toBeInstanceOf(Answer);
  expect(found.is('Number')).toBe(true);
  // `Answer` has a variant with no payload, so its `value` is a union and the
  // tag above is what narrows it — for `tsc`, said with the cast.
  expect((found.value as { _0: number })._0).toBe(7);
  found.drop();

  const missing = resolve(new Source('Absent', {}), null);
  expect(missing).toBeInstanceOf(Answer);
  expect(missing.is('Missing')).toBe(true);
  missing.drop();

  const given = resolve(new Source('Given', { _0: 3 }), null);
  expect((given.value as { _0: number })._0).toBe(3);
  given.drop();
});

test('a match nested two deep hands its value back at every level', () => {
  const floor = resolveTwice(new Source('Absent', {}), null, 2);
  expect(floor.is('Number')).toBe(true);
  expect((floor.value as { _0: number })._0).toBe(2);
  floor.drop();

  const nothing = resolveTwice(new Source('Absent', {}), null, null);
  expect(nothing.is('Missing')).toBe(true);
  nothing.drop();
});

test('an arm ending in a conditional jump stops the arms below it', () => {
  // The defective answer: `Light(5)` took the guarded arm — whose `if` does not
  // fire at 5 — and then fell into `Light(_)` as well, so `into` was [5, 0].
  const into: number[] = [];
  const five = new Weight('Light', { _0: 5 });
  expect(record(five, into)).toBe(0);
  expect(into).toEqual([5]);
  five.drop();

  // The `if` fires: the arm leaves the function outright.
  const huge = new Weight('Light', { _0: 200 });
  const bigger: number[] = [];
  expect(record(huge, bigger)).toBe(1);
  expect(bigger).toEqual([200]);
  huge.drop();

  // The guard fails, so the arm below it is the one that runs.
  const small = new Weight('Light', { _0: 2 });
  const low: number[] = [];
  expect(record(small, low)).toBe(0);
  expect(low).toEqual([0]);
  small.drop();

  const heavy = new Weight('Heavy', { _0: 9 });
  const rest: number[] = [];
  expect(record(heavy, rest)).toBe(0);
  expect(rest).toEqual([9]);
  heavy.drop();
});

test('a guarded consuming arm hands its value back', () => {
  expect(weigh(new Weight('Light', { _0: 9 }), 4)).toBe(9);
  expect(weigh(new Weight('Light', { _0: 1 }), 4)).toBe(4);
  expect(weigh(new Weight('Heavy', { _0: 6 }), 4)).toBe(12);
});

test('a consuming arm whose body is a nested match hands its value back', () => {
  // The defective answer: the `Two` arm's nested match stood there as bare
  // statements and the arm answered `undefined`.
  expect(pick(new Holder('Two', { _0: Token.new(8) }), 0)).toBe(8);
  expect(pick(new Holder('Two', { _0: Token.new(8) }), 3)).toBe(3);
  // The guarded `One` arm, and the arm below it.
  expect(pick(new Holder('One', { _0: Token.new(9) }), 4)).toBe(9);
  expect(pick(new Holder('One', { _0: Token.new(200) }), 4)).toBe(100);
  expect(pick(new Holder('One', { _0: Token.new(1) }), 4)).toBe(4);
});

test('a guarded consuming arm over a droppable payload releases it on every path', () => {
  expect(tally(new Source('Given', { _0: 5 }), Token.new(2), null)).toBe(7);
  expect(tally(new Source('Given', { _0: 0 }), Token.new(2), 4)).toBe(6);
  expect(tally(new Source('Given', { _0: 0 }), Token.new(2), null)).toBe(2);
  expect(tally(new Source('Absent', {}), Token.new(3), 9)).toBe(3);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
