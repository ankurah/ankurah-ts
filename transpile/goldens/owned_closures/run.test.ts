// Runs the emitted owned_closures against the real runtime. The question each
// function answers is who releases the captured Entity. Where the closure is
// bound to a local it has to be an OwnedClosure, because that is the only form
// whose captures the cascade can see — and it is invoked through `.call()`,
// never `f()`, so that the invocation goes through the liveness check. Where the
// closure is made and called in one statement, or captures nothing droppable, or
// only borrows, an ordinary function is right and the capture belongs to the
// block.

import { expect, test } from 'bun:test';
import { OwnedClosure } from '@ankurah/base';
import { Entity, borrow, borrowing, consumed, handsAPlainOne, handsAWrappedOne, plain, runLater, runNow, throughABound, twiceByReference, twiceByValue } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('abc');
  expect(borrow(entity)).toBe(3);
  entity.drop();
});

test('runNow releases the capture at the end of the statement that made it', () => {
  expect(runNow()).toBe(3);
});

test('runLater calls the closure twice and releases it after', () => {
  expect(runLater()).toBe(8);
});

test('runLater can run twice over, so no closure outlives its block', () => {
  runLater();
  expect(runLater()).toBe(8);
});

test('plain captures nothing droppable and stays an ordinary function', () => {
  expect(plain(4)).toBe(5);
});

test('borrowing leaves the Entity to the block, which releases it', () => {
  expect(borrowing()).toBe(2);
});

test('an OwnedClosure built here releases its captures when it is dropped', () => {
  const entity = new Entity('xyz');
  const closure = new OwnedClosure([entity], () => borrow(entity));
  expect(closure.call()).toBe(3);
  // Dropping the closure cascades into the Entity it captured; nothing else
  // may release that Entity.
  closure.drop();
});

test('a closure that hands its capture away is called once and releases it', () => {
  // The capture used to be left out of what the closure owned, and nothing
  // released it; `callOnce` transfers it and marks the closure moved, so the
  // closure is not dropped after one either.
  expect(consumed(new Entity('abcde'))).toBe(5);
});

test('a callee that sees only the bound calls either shape', () => {
  // The wrapped one: `through_a_bound` is written `f(n)` in Rust and cannot see
  // that this caller's closure captured an Entity, so it goes through `invoke`.
  expect(handsAWrappedOne(new Entity('abcde'))).toBe(6);
  expect(handsAPlainOne(4)).toBe(5);
  // And directly, with each shape.
  expect(throughABound((n: number) => n * 2, 3)).toBe(6);
  const entity = new Entity('xy');
  expect(
    throughABound(new OwnedClosure<[number], number>([entity], (n) => n + entity.name.length), 1),
  ).toBe(3);
});

// A callable parameter written BY VALUE is the body's: Rust drops it at the end
// of the body, and only the CALL borrows. The port wrote the call as
// `invokeRef`, which is right, and released nothing — so every capture of every
// wrapped closure handed to one leaked.
test('a by-value callable parameter is released by the body it was handed to', () => {
  const held = new Entity('abc');
  const f = new OwnedClosure<[number], number>([held], (n) => n + held.name.length);
  expect(twiceByValue(f, 1)).toBe(8);
  // Called twice — an `FnMut` bound, so the call borrows — and released once,
  // at the end of the body, which is where Rust drops it.
  expect(f.isDropped).toBe(true);
});

// The same bound written `&mut F`: the closure is the caller's, and a release
// written in the callee would drop a value somebody else still holds.
test('a by-reference callable parameter is left to its owner', () => {
  const held = new Entity('abc');
  const f = new OwnedClosure<[number], number>([held], (n) => n + held.name.length);
  expect(twiceByReference(f, 1)).toBe(8);
  expect(f.isDropped).toBe(false);
  // Still callable, which is the whole point of the reference form.
  expect(twiceByReference(f, 2)).toBe(10);
  f.drop();
});

// A plain function reaches none of `dropOwned`'s branches.
test('a plain function handed by value is left alone', () => {
  expect(twiceByValue((n: number) => n * 2, 3)).toBe(12);
  expect(twiceByReference((n: number) => n * 2, 3)).toBe(12);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
