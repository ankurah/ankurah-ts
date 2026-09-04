// Runs the emitted condition_guard_field against the real runtime. The check is
// the lock itself: both bodies take the lock the condition just took, and this
// runtime throws on a second lock rather than hanging the way Rust would. So a
// condition guard the emitter forgot to release turns straight into a thrown
// `Mutex already locked`, and a `while` whose condition was hoisted out of the
// loop either never runs or never stops.

import { expect, test } from 'bun:test';
import { Mutex } from '@ankurah/base';
import { Cell, Slot } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

/** Read a Cell's slot the way the emitted code does, and let the guard go. */
function read(cell: Cell): number {
  const guard = cell.slot.lock();
  try {
    return guard.value.n;
  } finally {
    guard.drop();
  }
}

test('clear locks again inside the body the condition locked for', () => {
  const cell = new Cell(new Mutex(new Slot(3)));
  expect(cell.clear()).toBe(true);
  expect(read(cell)).toBe(0);
  cell.drop();
});

test('clear takes the false path and still releases its condition guard', () => {
  const cell = new Cell(new Mutex(new Slot(0)));
  expect(cell.clear()).toBe(false);
  // Readable afterwards, which it would not be if the condition still held it.
  expect(read(cell)).toBe(0);
  cell.drop();
});

test('drain re-evaluates its condition every turn', () => {
  const cell = new Cell(new Mutex(new Slot(4)));
  expect(cell.drain()).toBe(4);
  expect(read(cell)).toBe(0);
  cell.drop();
});

test('drain on an already-empty cell runs no turn at all', () => {
  const cell = new Cell(new Mutex(new Slot(0)));
  expect(cell.drain()).toBe(0);
  cell.drop();
});

test('both methods leave the mutex free for the next caller', () => {
  const cell = new Cell(new Mutex(new Slot(2)));
  cell.drain();
  expect(cell.clear()).toBe(false);
  expect(read(cell)).toBe(0);
  cell.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
