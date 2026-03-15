// MIRRORS: ankurah/signals/src/signal/read.rs

import { describe, test, expect } from 'bun:test';
import { Mut } from './mutable.ts';

describe('Read', () => {
  test('reflects mutable value', () => {
    const mutable = new Mut(42);
    const read = mutable.read();

    expect(read.get()).toBe(42);

    mutable.set(100);
    expect(read.get()).toBe(100);
  });

  test('get and peek behave identically in Phase 1', () => {
    const mutable = new Mut('hello');
    const read = mutable.read();

    expect(read.get()).toBe('hello');
    expect(read.peek()).toBe('hello');

    mutable.set('world');
    expect(read.get()).toBe('world');
    expect(read.peek()).toBe('world');
  });

  test('with provides access to value', () => {
    const mutable = new Mut(10);
    const read = mutable.read();

    const result = read.with((v) => v * 2);
    expect(result).toBe(20);
  });

  test('listen notifies on change', () => {
    const mutable = new Mut(0);
    const read = mutable.read();
    let notified = false;

    const guard = read.listen(() => {
      notified = true;
    });

    mutable.set(1);
    expect(notified).toBe(true);

    guard.drop();
  });

  test('listen guard unsubscribes', () => {
    const mutable = new Mut(0);
    const read = mutable.read();
    let count = 0;

    const guard = read.listen(() => {
      count++;
    });

    mutable.set(1);
    expect(count).toBe(1);

    guard.drop();

    mutable.set(2);
    expect(count).toBe(1); // Not called again
  });

  test('subscribe receives updated values', () => {
    const mutable = new Mut(0);
    const read = mutable.read();
    const received: number[] = [];

    const subscription = read.subscribe((value) => {
      received.push(value);
    });

    mutable.set(1);
    mutable.set(2);
    mutable.set(3);

    expect(received).toEqual([1, 2, 3]);

    subscription.drop();
  });

  test('subscribe does not fire on initial value', () => {
    const mutable = new Mut(42);
    const read = mutable.read();
    let called = false;

    const subscription = read.subscribe(() => {
      called = true;
    });

    expect(called).toBe(false);

    subscription.drop();
  });

  test('broadcastId matches parent mutable', () => {
    const mutable = new Mut(0);
    const read = mutable.read();
    expect(read.broadcastId().equals(mutable.broadcastId())).toBe(true);
  });

  test('multiple read instances share same broadcast', () => {
    const mutable = new Mut(0);
    const read1 = mutable.read();
    const read2 = mutable.read();

    expect(read1.broadcastId().equals(read2.broadcastId())).toBe(true);
  });

  test('getReadCell provides shared access', () => {
    const mutable = new Mut(42);
    const read = mutable.read();
    const cell = read.getReadCell();

    expect(cell.getValue()).toBe(42);

    mutable.set(100);
    expect(cell.getValue()).toBe(100);
  });

  test('subscription guard drop stops notifications', () => {
    const mutable = new Mut(0);
    const read = mutable.read();
    const received: number[] = [];

    const subscription = read.subscribe((value) => {
      received.push(value);
    });

    mutable.set(1);
    expect(received).toEqual([1]);

    subscription.drop();

    mutable.set(2);
    expect(received).toEqual([1]); // No new values after drop
  });
});
