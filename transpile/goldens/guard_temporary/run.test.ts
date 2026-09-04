// Runs the emitted guard_temporary against the real runtime. What is under test
// is the lock: a guard nothing binds must be released at the end of its
// statement and still be safe to release again in the enclosing `finally`, and
// the Mutex must be lockable again afterwards — a guard left holding it makes
// the next lock() throw.

import { expect, test } from 'bun:test';
import { Mutex } from '@ankurah/base';
import { Counter } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('read releases the unbound guard, so the Mutex can be locked again', () => {
  const counter = new Counter(new Mutex(1));
  expect(counter.read()).toBe(2);
  expect(counter.read()).toBe(2);
  counter.drop();
});

test('bump writes through its guard and releases it', () => {
  const counter = new Counter(new Mutex(0));
  expect(counter.bump()).toBe(1);
  expect(counter.bump()).toBe(2);
  counter.drop();
});

test('read sees what bump wrote, so both guards reach the same storage', () => {
  const counter = new Counter(new Mutex(0));
  counter.bump();
  expect(counter.read()).toBe(2);
  counter.drop();
});

test('dropping the Counter releases the Mutex it owns', () => {
  const counter = new Counter(new Mutex(7));
  counter.drop();
  // A second lock() here would report a use-after-drop, which is the check.
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
