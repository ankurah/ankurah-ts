// Runs the emitted arc_mutex_field against the real runtime. What is under test
// is the guard in `setLabel`: `*self.0.label.lock().unwrap() = label` produces a
// guard nothing binds, and this runtime throws `Mutex already locked` on the
// next lock if that guard was never released. `labelLen` after `setLabel` is the
// check, and it is not incidental — the emitted output used to read
// `this._0.value.label.lock().value = label` and held the lock forever.

import { expect, test } from 'bun:test';
import { Counter } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('labelLen reads through the Arc and the guard, and lets the guard go', () => {
  const counter = Counter.new('abc');
  expect(counter.labelLen()).toBe(3);
  // Readable a second time, which it would not be if the first guard survived.
  expect(counter.labelLen()).toBe(3);
  counter.drop();
});

test('setLabel writes through its guard and releases it', () => {
  const counter = Counter.new('a');
  counter.setLabel('abcd');
  expect(counter.labelLen()).toBe(4);
  counter.drop();
});

test('setLabel can be called again, so it left the mutex free', () => {
  const counter = Counter.new('a');
  counter.setLabel('bb');
  counter.setLabel('ccc');
  expect(counter.labelLen()).toBe(3);
  counter.drop();
});

test('dropping the Counter releases the Arc and the Mutex inside it', () => {
  const counter = Counter.new('xyz');
  expect(counter.labelLen()).toBe(3);
  counter.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
