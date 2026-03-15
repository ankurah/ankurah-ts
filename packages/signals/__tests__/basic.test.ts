// MIRRORS: ankurah/signals/tests/basic.rs
//
// All 15 Rust test functions ported.

import { describe, test, expect } from 'bun:test';
import { Mut, Map, Memo, waitValue, waitFor } from '../src/index.ts';

/**
 * Helper: watcher that collects values (port of tests/common.rs watcher)
 */
function watcher<T>(): [(value: T) => void, () => T[]] {
  const values: T[] = [];
  const accumulate = (value: T) => {
    values.push(value);
  };
  const check = () => {
    const result = values.splice(0);
    return result;
  };
  return [accumulate, check];
}

describe('basic signal tests (from tests/basic.rs)', () => {
  // Rust: async fn test_basic_signal()
  test('test_basic_signal', () => {
    const mutable = new Mut(42);
    const read = mutable.read();

    // closure subscription
    const [w, check] = watcher<number>();
    const _handle = read.subscribe(w);

    mutable.set(43);
    mutable.set(44);

    // Signals are only notified on updates, not initial value
    // Divergence: TS notifications are synchronous, no sleep needed [E8]
    expect(check()).toEqual([43, 44]);
  });

  // Rust: async fn test_basic_subscriber()
  test('test_basic_subscriber', () => {
    const mutable = new Mut(42);
    const read = mutable.read();

    const received: number[] = [];
    const handle = read.subscribe((value: number) => {
      received.push(value);
    });

    mutable.set(43);
    expect(received).toEqual([43]);

    handle.drop(); // unsubscribe
  });

  // Rust: #[cfg(feature = "tokio")] async fn test_wait_value()
  test('test_wait_value', async () => {
    const mutable = new Mut(1);
    const read = mutable.read();

    // Test immediate return when value already matches
    await waitValue(read, 1);

    // Test waiting for a future value
    // Divergence: Rust uses tokio::spawn; TS uses setTimeout to defer the set [E8]
    const task = waitValue(read, 42);
    setTimeout(() => mutable.set(42), 10);

    await task;
  });

  // Rust: async fn test_wait_predicate()
  test('test_wait_predicate', async () => {
    const mutable = new Mut(1);
    const read = mutable.read();

    // Test waiting for a value matching a predicate
    const task = waitFor(read, (v: number) => v > 10);

    setTimeout(() => mutable.set(15), 10);

    await task;
  });

  // Rust: async fn test_wait_for_result()
  test('test_wait_for_result', async () => {
    // Divergence: Rust uses enum State { Loading, Success(String), Error(String) };
    // TS uses discriminated union [E8]
    type State =
      | { type: 'Loading' }
      | { type: 'Success'; data: string }
      | { type: 'Error'; msg: string };

    const mutable = new Mut<State>({ type: 'Loading' });
    const read = mutable.read();

    // Test waiting with Option<Result> return type
    // Divergence: Rust returns Result<String, String>; TS returns { ok: string } | { err: string } | null [E8]
    const successTask = waitFor(read, (state: State): string | null => {
      switch (state.type) {
        case 'Success': return state.data;
        case 'Error': return null; // We'll test error separately
        case 'Loading': return null;
      }
    });

    setTimeout(() => mutable.set({ type: 'Success', data: 'completed' }), 10);

    const result = await successTask;
    expect(result).toBe('completed');

    // Test error case
    mutable.set({ type: 'Loading' }); // Reset

    const errorTask = waitFor(read, (state: State): string | null => {
      switch (state.type) {
        case 'Success': return state.data;
        case 'Error': return `Failed: ${state.msg}`;
        case 'Loading': return null;
      }
    });

    setTimeout(() => mutable.set({ type: 'Error', msg: 'network timeout' }), 10);

    const errorResult = await errorTask;
    expect(errorResult).toBe('Failed: network timeout');
  });

  // Rust: async fn test_wait_for_boolean()
  test('test_wait_for_boolean', async () => {
    const mutable = new Mut(1);
    const read = mutable.read();

    const task = waitFor(read, (value: number) => value > 5);

    setTimeout(() => mutable.set(10), 10);

    await task;
  });

  // Rust: async fn test_wait_for_option()
  test('test_wait_for_option', async () => {
    const mutable = new Mut(5);
    const read = mutable.read();

    // Wait for non-zero remainder when divided by 5
    const task = waitFor(read, (value: number): number | null => {
      const rem = value % 5;
      if (rem === 0) return null;
      return rem;
    });

    setTimeout(() => mutable.set(7), 10);

    const remainder = await task;
    expect(remainder).toBe(2); // 7 % 5 = 2
  });

  // Rust: async fn test_wait_for_immediate_match()
  test('test_wait_for_immediate_match', async () => {
    const mutable = new Mut(10);
    const read = mutable.read();

    // Should return immediately since condition is already met
    await waitFor(read, (value: number) => value > 5);

    // Should return immediately with extracted value
    const remainder = await waitFor(read, (value: number): number | null => {
      const rem = value % 7;
      if (rem === 0) return null;
      return rem;
    });

    expect(remainder).toBe(3); // 10 % 7 = 3
  });

  // Rust: async fn test_map_signal()
  test('test_map_signal', () => {
    const mutable = new Mut(10);
    const mapped = new Map(mutable.read(), (x: number) => x * 2);

    // Test With trait
    mapped.with((val) => expect(val).toBe(20));

    // Test Get trait
    expect(mapped.get()).toBe(20);

    // Test subscription
    const [w, check] = watcher<number>();
    const _handle = mapped.subscribe(w);

    mutable.set(15);
    mutable.set(20);

    // Divergence: TS notifications are synchronous, no sleep needed [E8]
    // Should receive transformed values
    expect(check()).toEqual([30, 40]); // 15*2=30, 20*2=40
  });

  // Rust: async fn test_map_signal_string_transform()
  test('test_map_signal_string_transform', () => {
    const mutable = new Mut(5);
    const mapped = new Map(mutable.read(), (x: number) => `Value: ${x}`);

    // Test With trait with type transformation
    mapped.with((val) => expect(val).toBe('Value: 5'));

    // Test subscription with type transformation
    const received: string[] = [];
    const _handle = mapped.subscribe((v) => received.push(v));

    mutable.set(10);
    mutable.set(15);

    // Divergence: TS uses callback array instead of mpsc channel [E8]
    expect(received[0]).toBe('Value: 10');
    expect(received[1]).toBe('Value: 15');
  });

  // Rust: async fn test_read_map_convenience_method()
  test('test_read_map_convenience_method', () => {
    const mutable = new Mut(100);
    const read = mutable.read();

    // Use the convenience method to create a mapped signal
    const doubled = read.map((x: number) => x * 2);
    const stringified = read.map((x: number) => `Number: ${x}`);

    // Test both mapped signals
    doubled.with((val) => expect(val).toBe(200));
    stringified.with((val) => expect(val).toBe('Number: 100'));

    // Test subscription on mapped signals
    const received1: number[] = [];
    const received2: string[] = [];
    const _handle1 = doubled.subscribe((v) => received1.push(v));
    const _handle2 = stringified.subscribe((v) => received2.push(v));

    mutable.set(50);

    // Should receive transformed values
    expect(received1[0]).toBe(100); // 50 * 2
    expect(received2[0]).toBe('Number: 50');
  });

  // Rust: async fn test_memo_caches_value()
  test('test_memo_caches_value', () => {
    const mutable = new Mut(10);
    let transformCount = 0;

    const memo = mutable.read().memo((x: number) => {
      transformCount++;
      return x * 2;
    });

    // First access computes the value
    expect(memo.get()).toBe(20);
    expect(transformCount).toBe(1);

    // Subsequent accesses return cached value without recomputing
    expect(memo.get()).toBe(20);
    expect(memo.get()).toBe(20);
    expect(transformCount).toBe(1); // Still 1
  });

  // Rust: async fn test_memo_invalidates_on_change()
  test('test_memo_invalidates_on_change', () => {
    const mutable = new Mut(10);
    let transformCount = 0;

    const memo = mutable.read().memo((x: number) => {
      transformCount++;
      return x * 2;
    });

    // First access
    expect(memo.get()).toBe(20);
    expect(transformCount).toBe(1);

    // Change upstream - cache should be invalidated
    // Divergence: TS notifications are synchronous, no sleep needed [E8]
    mutable.set(15);

    // Next access recomputes
    expect(memo.get()).toBe(30);
    expect(transformCount).toBe(2);

    // Subsequent accesses return cached value
    expect(memo.get()).toBe(30);
    expect(transformCount).toBe(2); // Still 2
  });

  // Rust: async fn test_memo_subscription()
  test('test_memo_subscription', () => {
    const mutable = new Mut(5);
    const memo = mutable.read().memo((x: number) => `Value: ${x}`);

    // Divergence: TS uses callback array instead of mpsc channel [E8]
    const received: string[] = [];
    const _handle = memo.subscribe((v) => received.push(v));

    mutable.set(10);
    mutable.set(15);

    expect(received[0]).toBe('Value: 10');
    expect(received[1]).toBe('Value: 15');
  });

  // Rust: async fn test_memo_with_does_not_require_clone()
  test('test_memo_with_does_not_require_clone', () => {
    // Divergence: JS objects are always reference types, Clone is N/A.
    // Test still verifies .with() works with non-primitive output [E8].
    class NonClone {
      constructor(public readonly value: number) {}
    }

    const mutable = new Mut(10);
    const memo = mutable.read().memo((x: number) => new NonClone(x * 2));

    // with() should work without Clone
    memo.with((val) => expect(val.value).toBe(20));

    mutable.set(15);

    memo.with((val) => expect(val.value).toBe(30));
  });
});
