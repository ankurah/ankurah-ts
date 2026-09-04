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
import { Entity, borrow, borrowing, plain, runLater, runNow } from './input.ts';
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

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
