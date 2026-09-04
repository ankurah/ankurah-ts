// Runs the emitted param_and_self against the real runtime. What is under test
// is who owes the drop after a by-value hand-over. `forward` owes it only on the
// path that kept the Entity, which is what the drop flag decides. `intoInner`
// hands one field to the caller and still owes a drop on the receiver, whose
// cascade must reach the field it kept and not the one it gave away. `width`
// owes nothing, because `&self` never took the receiver in the first place.

import { expect, test } from 'bun:test';
import { Entity, Holder, borrow, consume, forward } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('consume takes an Entity by value and releases it', () => {
  expect(consume(new Entity('ab'))).toBe(2);
});

test('forward hands the Entity on where the flag is set', () => {
  expect(forward(new Entity('abc'), true)).toBe(3);
});

test('forward keeps and releases the Entity where the flag is not set', () => {
  expect(forward(new Entity('abcd'), false)).toBe(4);
});

test('forward can be called either way in turn, with one flag per call', () => {
  expect(forward(new Entity('a'), false)).toBe(1);
  expect(forward(new Entity('bb'), true)).toBe(2);
  expect(forward(new Entity('ccc'), false)).toBe(3);
});

test('width borrows through the receiver and leaves it to its owner', () => {
  const holder = new Holder(new Entity('ab'), new Entity('cde'));
  expect(holder.width()).toBe(5);
  // Readable again: width took nothing.
  expect(holder.width()).toBe(5);
  holder.drop();
});

test('intoInner hands the field out and releases the receiver holding the rest', () => {
  const holder = new Holder(new Entity('kept'), new Entity('gone'));
  const inner = holder.intoInner();
  expect(inner.name).toBe('kept');
  // The field that came out is live and is now ours.
  expect(borrow(inner)).toBe(4);
  inner.drop();
});

test('widthOwned releases the whole receiver, both fields included', () => {
  expect(new Holder(new Entity('abc'), new Entity('de')).widthOwned()).toBe(3);
});

test('a Holder nobody consumes releases both fields', () => {
  const holder = new Holder(new Entity('x'), new Entity('yy'));
  expect(borrow(holder.inner)).toBe(1);
  holder.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
