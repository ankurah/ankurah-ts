// Runs the emitted condition_chain_temporaries against the real runtime. Two
// things are under test. Every `Reading` a condition builds must reach a drop,
// which the leak registry is what catches; and the lock the third condition
// takes must be released before the caller looks at the Meter again, which this
// runtime reports as `Mutex already locked` when it is not.
//
// A condition that is never reached must also never run: `band(20)` takes the
// first branch, so the second `Reading` is never built and the lock is never
// taken at all.

import { expect, test } from 'bun:test';
import { Mutex } from '@ankurah/base';
import { Meter, Reading, reading } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

/** Read a Meter's floor the way the emitted code does, and let the guard go. */
function floorOf(meter: Meter): number {
  const guard = meter.floor.lock();
  try {
    return guard.value;
  } finally {
    guard.drop();
  }
}

test('reading builds a Reading the caller then owns', () => {
  const built: Reading = reading(4);
  expect(built.level).toBe(4);
  built.drop();
});

test('band takes the first branch and never evaluates the conditions below it', () => {
  const meter = new Meter(new Mutex(0));
  expect(meter.band(20)).toBe(3);
  expect(floorOf(meter)).toBe(0);
  meter.drop();
});

test('band falls to the second condition, which builds its own Reading', () => {
  const meter = new Meter(new Mutex(0));
  expect(meter.band(7)).toBe(2);
  meter.drop();
});

test('band reaches the guarded condition and releases the guard either way', () => {
  const meter = new Meter(new Mutex(5));
  expect(meter.band(1)).toBe(1);
  // Lockable again, which it would not be if the condition still held it.
  expect(floorOf(meter)).toBe(5);
  expect(meter.band(5)).toBe(0);
  expect(floorOf(meter)).toBe(5);
  meter.drop();
});

test('band survives being walked down every branch many times over', () => {
  const meter = new Meter(new Mutex(3));
  for (let level = 0; level < 15; level += 1) {
    meter.band(level);
  }
  expect(floorOf(meter)).toBe(3);
  meter.drop();
});

test('climb releases the Reading its condition built on every turn', () => {
  const meter = new Meter(new Mutex(0));
  expect(meter.climb()).toBe(3);
  meter.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
