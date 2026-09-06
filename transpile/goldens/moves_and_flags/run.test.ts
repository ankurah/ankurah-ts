// Runs the emitted moves_and_flags against the real runtime. What is under test
// is who releases each Entity: a callee that took it by value, the block that
// still owns it, the drop flag on the path that hands it away, or the caller who
// received it.

import { expect, test } from 'bun:test';
import {
  Entity,
  Pair,
  borrow,
  borrowedByACall,
  consume,
  droppedByHand,
  movedIntoACall,
  movedIntoALiteral,
  movedOnOnePath,
  Sink,
  constructor as build,
  methodCall,
  plainCall,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('consume takes the Entity by value and releases it', () => {
  expect(consume(new Entity('ab'))).toBe(2);
});

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('abc');
  expect(borrow(entity)).toBe(3);
  entity.drop();
});

test('movedIntoACall hands its local away, so the block does not release it', () => {
  expect(movedIntoACall()).toBe(0);
});

test('borrowedByACall keeps its local and releases it', () => {
  expect(borrowedByACall()).toBe(0);
});

test('movedIntoALiteral hands the Entity to the Pair it returns', () => {
  const pair = movedIntoALiteral();
  expect(pair.left.name).toBe('');
  // Dropping the Pair cascades into the Entity it now owns.
  pair.drop();
});

test('a Pair built here owns its Entity the same way', () => {
  const pair = new Pair(new Entity('xy'));
  expect(pair.left.name).toBe('xy');
  pair.drop();
});

test('movedOnOnePath releases the Entity only on the path that kept it', () => {
  expect(movedOnOnePath(true)).toBe(0);
  expect(movedOnOnePath(false)).toBe(0);
});

test('droppedByHand releases where the source says and the block does not repeat it', () => {
  expect(droppedByHand()).toBe(0);
});

// E10: the move flag stands after everything the statement evaluates, whatever
// SHAPE the call is. At the parent it was written before the whole statement
// for every call but `invoke(..)`, so an argument that throws left the flag set
// and the moved value released by nobody — the leak check below catches it.
test('an argument that throws leaves the moved value releasable', () => {
  const sink = new Sink();
  for (const call of [
    () => plainCall(new Entity('a'), null, false),
    () => methodCall(sink, new Entity('a'), null, false),
    () => build(new Entity('a'), null, false),
  ]) {
    expect(call).toThrow('called `Option::unwrap()` on a `None` value');
  }
  sink.drop();
});

// And the call that does not throw still hands the value over, so the block
// releases nothing.
test('a call that completes still moves what it was given', () => {
  const sink = new Sink();
  expect(plainCall(new Entity('ab'), 1, false)).toBe(3);
  expect(methodCall(sink, new Entity('ab'), 1, false)).toBe(3);
  const held = build(new Entity('ab'), 1, false);
  expect(held.n).toBe(1);
  held.drop();
  sink.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
