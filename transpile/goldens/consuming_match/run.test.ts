// Runs the emitted consuming_match against the real runtime. What is under test
// is which of the two match forms each function got. `intoMatch` marks the enum
// moved and takes the payload out of the cascade, so an arm that hands the
// payload on leaves nothing to release, and an arm that only reads it must
// release it itself. `match` leaves the enum whole, so the caller still owes it
// a drop — and a `peek` emitted as `intoMatch` by mistake would turn the drop
// after it into a fatal use-after-move.

import { expect, test } from 'bun:test';
import { Entity, Slot, borrow, consume, intoEntity, peek, take, width, label } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('consume takes an Entity by value and releases it', () => {
  expect(consume(new Entity('ab'))).toBe(2);
});

test('borrow leaves the Entity to its owner', () => {
  const entity = new Entity('abc');
  expect(borrow(entity)).toBe(3);
  entity.drop();
});

test('take hands the payload to the callee and leaves the Slot moved', () => {
  expect(take(new Slot('Filled', { _0: new Entity('abcd') }))).toBe(4);
});

test('take on the empty variant releases the Slot it was handed', () => {
  expect(take(new Slot('Empty', {}))).toBe(0);
});

test('width keeps the payload in the arm, so the arm releases it', () => {
  expect(width(new Slot('Filled', { _0: new Entity('ab') }))).toBe(3);
  expect(width(new Slot('Empty', {}))).toBe(0);
});

test('intoEntity hands the payload out to the caller', () => {
  const entity = intoEntity(new Slot('Filled', { _0: new Entity('xyz') }));
  expect(entity).not.toBeNull();
  expect(entity!.name).toBe('xyz');
  entity!.drop();
  expect(intoEntity(new Slot('Empty', {}))).toBeNull();
});

test('peek borrows, so the Slot survives the call and is ours to release', () => {
  const slot = new Slot('Filled', { _0: new Entity('hello') });
  expect(peek(slot)).toBe(5);
  // Readable a second time: peek moved nothing.
  expect(peek(slot)).toBe(5);
  slot.drop();
});

test('peek on the empty variant leaves the Slot whole too', () => {
  const slot = new Slot('Empty', {});
  expect(peek(slot)).toBe(0);
  slot.drop();
});

test('an arm with a block body produces its tail and still releases the payload', () => {
  expect(label(new Slot('Filled', { _0: new Entity('abc') }))).toBe(6);
  expect(label(new Slot('Empty', {}))).toBe(0);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
