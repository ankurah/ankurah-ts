// Runs the emitted condition_temporaries against the real runtime. The check is
// the same in both functions and it is not subtle: the body locks the mutex the
// condition just locked, and this runtime throws on a second lock rather than
// hanging the way Rust would. So a condition guard the emitter forgot to release
// turns straight into a thrown `Mutex already locked`, and a `while` whose
// condition the emitter hoisted out of the loop either never runs or never
// stops.

import { expect, test } from 'bun:test';
import { Mutex } from '@ankurah/base';
import { Counter } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

/** Read a Counter's value the way the emitted code does, and let the guard go. */
function read(counter: Counter): number {
  const guard = counter.value.lock();
  try {
    return guard.value;
  } finally {
    guard.drop();
  }
}

test('startIfIdle locks again inside the body the condition locked for', () => {
  const counter = new Counter(new Mutex(0));
  expect(counter.startIfIdle()).toBe(true);
  expect(read(counter)).toBe(1);
  counter.drop();
});

test('startIfIdle takes the false path and still releases its condition guard', () => {
  const counter = new Counter(new Mutex(4));
  expect(counter.startIfIdle()).toBe(false);
  expect(read(counter)).toBe(4);
  counter.drop();
});

test('windDown re-evaluates its condition every turn', () => {
  const counter = new Counter(new Mutex(3));
  expect(counter.windDown()).toBe(3);
  expect(read(counter)).toBe(0);
  counter.drop();
});

test('windDown on an already-empty counter runs no turn at all', () => {
  const counter = new Counter(new Mutex(0));
  expect(counter.windDown()).toBe(0);
  counter.drop();
});

test('both functions leave the mutex free for the next caller', () => {
  const counter = new Counter(new Mutex(2));
  counter.startIfIdle();
  counter.windDown();
  expect(counter.startIfIdle()).toBe(true);
  expect(read(counter)).toBe(1);
  counter.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
