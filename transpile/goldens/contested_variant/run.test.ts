// Runs the emitted contested_variant against the real runtime. Every one of
// these answered the FIRST arm naming the variant, whatever the value inside
// the payload was: `truthy(Literal(Flag(false)))` answered `Ok(true)`, and
// `takeOne(Ready(None))` read the end of the queue as an item. The chain is
// what makes the inner patterns tested, so these are the answers Rust gives.

import { expect, test } from 'bun:test';
import { Expr, Lit, Payload, Step, describe, drain, takeOne, truthy, widen, widest } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

const flag = (b: boolean) => new Expr('Literal', { _0: new Lit('Flag', { _0: b }) });
const count = (n: number) => new Expr('Literal', { _0: new Lit('Count', { _0: n }) });

test('a variant two arms name answers the arm whose pattern matched', () => {
  expect(truthy(flag(true)).unwrap()).toBe(true);
  // The one the old key could not reach: `false` used to answer `Ok(true)`.
  expect(truthy(flag(false)).unwrap()).toBe(false);
  // `is_err` takes `&self`, so each of these Results is still the driver's.
  const onCount = truthy(count(3));
  expect(onCount.isErr()).toBe(true);
  onCount.drop();
  const onNothing = truthy(new Expr('Nothing', {}));
  expect(onNothing.isErr()).toBe(true);
  onNothing.drop();
});

test('the end of a stream is the end, not an item', () => {
  const into: number[] = [];
  expect(takeOne(new Step('Ready', { _0: new Payload(7) }), into)).toBe(true);
  expect(into).toEqual([7]);
  // `Ready(None)` used to take the first arm and read `null` as an item.
  expect(takeOne(new Step('Ready', { _0: null }), into)).toBe(false);
  expect(takeOne(new Step('Pending', {}), into)).toBe(false);
  expect(into).toEqual([7]);
});

test('a borrowed subject is read, not taken apart', () => {
  const e = flag(true);
  expect(describe(e)).toBe('flag');
  expect(e.widthOf()).toBe(1);
  e.drop();
  const c = count(4);
  expect(describe(c)).toBe('count');
  expect(c.widthOf()).toBe(4);
  expect(widest(c, c)).toBe(4);
  c.drop();
  const h = new Expr('Held', { _0: new Payload(9) });
  expect(describe(h)).toBe('held');
  expect(h.widthOf()).toBe(9);
  h.drop();
});

test('a `?` and an early return inside a branch leave the function', () => {
  const source = count(5);
  expect(widen(flag(true), source).unwrap()).toBe(6);
  // The early `return` in the second branch.
  expect(widen(flag(false), source).unwrapErr()).toBe('false');
  expect(widen(count(2), source).unwrap()).toBe(2);
  expect(widen(new Expr('Nothing', {}), source).unwrapErr()).toBe('no');
  // The `?`: `width` refuses a flag, and the refusal is the function's.
  const noWidth = flag(true);
  expect(widen(flag(true), noWidth).unwrapErr()).toBe('no width');
  noWidth.drop();
  source.drop();
});

test('a subject that is a call is evaluated once', () => {
  const items = [new Payload(1), new Payload(2), new Payload(3)];
  const into: number[] = [];
  // Three items, three turns: a subject read twice would take six.
  expect(drain(items, into)).toBe(3);
  expect(into).toEqual([3, 2, 1]);
  expect(items.length).toBe(0);
});

test('nothing leaked and nothing was dropped twice', async () => {
  // The recorded leak is gone: `widen`'s `Ex::Literal(Lit::Count(n))` link takes
  // a `u32` out of the `Lit`, so nothing droppable came out of it and the `Lit`
  // is whole — and `dropUnbound(v, [])` releases it.
  await expectNoOwnershipReports();
});
