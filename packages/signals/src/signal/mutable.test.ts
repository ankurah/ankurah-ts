// MIRRORS: ankurah/signals/src/signal/mutable.rs

import { describe, test, expect } from 'bun:test';
import { Mut } from './mutable.ts';

describe('Mut', () => {
  test('initial value', () => {
    const signal = new Mut(42);
    expect(signal.get()).toBe(42);
    expect(signal.peek()).toBe(42);
  });

  test('set updates value', () => {
    const signal = new Mut(42);
    signal.set(100);
    expect(signal.get()).toBe(100);
  });

  test('with provides access to value', () => {
    const signal = new Mut('hello');
    const result = signal.with((v) => v.toUpperCase());
    expect(result).toBe('HELLO');
  });

  test('listen notifies on change', () => {
    const signal = new Mut(0);
    let notified = false;

    const guard = signal.listen(() => {
      notified = true;
    });

    signal.set(1);
    expect(notified).toBe(true);

    guard.drop();
  });

  test('listen guard unsubscribes', () => {
    const signal = new Mut(0);
    let count = 0;

    const guard = signal.listen(() => {
      count++;
    });

    signal.set(1);
    expect(count).toBe(1);

    guard.drop();

    signal.set(2);
    expect(count).toBe(1); // Not called again
  });

  test('read returns read-only signal', () => {
    const mutable = new Mut(42);
    const read = mutable.read();

    expect(read.get()).toBe(42);

    mutable.set(100);
    expect(read.get()).toBe(100);
  });

  test('broadcastId is consistent', () => {
    const signal = new Mut(0);
    const id1 = signal.broadcastId();
    const id2 = signal.broadcastId();
    expect(id1.equals(id2)).toBe(true);
  });

  test('read shares broadcastId with mutable', () => {
    const mutable = new Mut(0);
    const read = mutable.read();
    expect(mutable.broadcastId().equals(read.broadcastId())).toBe(true);
  });

  test('subscribe receives updated values', () => {
    // Port of Rust test_subscribe_trait from broadcast.rs
    const signal = new Mut(42);
    let callCount = 0;

    const subscription = signal.subscribe(() => {
      callCount++;
    });

    signal.set(100);
    expect(callCount).toBe(1);

    subscription.drop();
  });

  test('subscribe receives correct value', () => {
    const signal = new Mut(0);
    const received: number[] = [];

    const subscription = signal.subscribe((value) => {
      received.push(value);
    });

    signal.set(1);
    signal.set(2);
    signal.set(3);

    expect(received).toEqual([1, 2, 3]);

    subscription.drop();
  });

  test('subscribe does not fire on initial value', () => {
    const signal = new Mut(42);
    let called = false;

    const subscription = signal.subscribe(() => {
      called = true;
    });

    expect(called).toBe(false);

    subscription.drop();
  });

  test('multiple listeners', () => {
    const signal = new Mut(0);
    let count1 = 0;
    let count2 = 0;

    const guard1 = signal.listen(() => { count1++; });
    const guard2 = signal.listen(() => { count2++; });

    signal.set(1);
    expect(count1).toBe(1);
    expect(count2).toBe(1);

    guard1.drop();

    signal.set(2);
    expect(count1).toBe(1); // Not called again
    expect(count2).toBe(2);

    guard2.drop();
  });

  test('getReadCell provides shared access', () => {
    const signal = new Mut(42);
    const cell = signal.getReadCell();

    expect(cell.getValue()).toBe(42);

    signal.set(100);
    expect(cell.getValue()).toBe(100);
  });

  test('works with object values', () => {
    const signal = new Mut({ name: 'Alice', age: 30 });
    expect(signal.get()).toEqual({ name: 'Alice', age: 30 });

    signal.set({ name: 'Bob', age: 25 });
    expect(signal.get()).toEqual({ name: 'Bob', age: 25 });
  });

  test('works with array values', () => {
    const signal = new Mut([1, 2, 3]);
    expect(signal.get()).toEqual([1, 2, 3]);

    signal.set([4, 5, 6]);
    expect(signal.get()).toEqual([4, 5, 6]);
  });
});
