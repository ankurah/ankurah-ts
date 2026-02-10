// MIRRORS: ankurah/signals/tests/basic.rs
//
// Only tests for implemented features are ported.
// Deferred: test_wait_value, test_wait_predicate, test_wait_for_result,
//           test_wait_for_boolean, test_wait_for_option, test_wait_for_immediate_match
//           (all require Wait<T> which needs async/tokio)
// Deferred: test_map_signal, test_map_signal_string_transform, test_read_map_convenience_method
//           (require Map<...> signal)
// Deferred: test_memo_caches_value, test_memo_invalidates_on_change, test_memo_subscription,
//           test_memo_with_does_not_require_clone (require Memo<...> signal)

import { describe, test, expect } from 'bun:test';
import { Mut } from '../src/index.ts';

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
  test('test_basic_signal', () => {
    const mutable = new Mut(42);
    const read = mutable.read();

    // closure subscription
    const [w, check] = watcher<number>();
    const _handle = read.subscribe(w);

    mutable.set(43);
    mutable.set(44);

    // Signals are only notified on updates, not initial value
    expect(check()).toEqual([43, 44]);
  });

  test('test_basic_subscriber', () => {
    const mutable = new Mut(42);
    const read = mutable.read();

    const received: number[] = [];
    const handle = read.subscribe((value: number) => {
      received.push(value);
    });

    mutable.set(43);
    expect(received).toEqual([43]);

    handle.dispose(); // unsubscribe
  });

  test('signal Mut subscribe/set integration', () => {
    // Additional integration test: ensure Mut.subscribe works directly
    const mutable = new Mut(0);
    const [w, check] = watcher<number>();
    const _sub = mutable.subscribe(w);

    mutable.set(1);
    mutable.set(2);
    mutable.set(3);

    expect(check()).toEqual([1, 2, 3]);
  });

  test('multiple subscriptions on same signal', () => {
    const mutable = new Mut(0);
    const read = mutable.read();

    const values1: number[] = [];
    const values2: number[] = [];

    const sub1 = read.subscribe((v) => values1.push(v));
    const sub2 = read.subscribe((v) => values2.push(v));

    mutable.set(10);
    expect(values1).toEqual([10]);
    expect(values2).toEqual([10]);

    sub1.dispose();

    mutable.set(20);
    expect(values1).toEqual([10]); // No longer receiving
    expect(values2).toEqual([10, 20]);

    sub2.dispose();
  });

  test('signal with complex types', () => {
    interface State {
      loading: boolean;
      data: string | null;
    }

    const mutable = new Mut<State>({ loading: true, data: null });
    const read = mutable.read();

    const states: State[] = [];
    const _sub = read.subscribe((s) => states.push(s));

    mutable.set({ loading: false, data: 'hello' });
    mutable.set({ loading: false, data: 'world' });

    expect(states).toEqual([
      { loading: false, data: 'hello' },
      { loading: false, data: 'world' },
    ]);
  });

  test('read signal reflects mutations through get/peek/with', () => {
    const mutable = new Mut(1);
    const read = mutable.read();

    expect(read.get()).toBe(1);
    expect(read.peek()).toBe(1);
    expect(read.with((v) => v + 10)).toBe(11);

    mutable.set(5);

    expect(read.get()).toBe(5);
    expect(read.peek()).toBe(5);
    expect(read.with((v) => v + 10)).toBe(15);
  });

  test('listener guard dispose is idempotent', () => {
    const mutable = new Mut(0);
    const read = mutable.read();
    let count = 0;

    const guard = read.listen(() => { count++; });

    mutable.set(1);
    expect(count).toBe(1);

    // Dispose multiple times
    guard.dispose();
    guard.dispose();

    mutable.set(2);
    expect(count).toBe(1);
  });

  test('subscription guard dispose is idempotent', () => {
    const mutable = new Mut(0);
    const received: number[] = [];

    const sub = mutable.subscribe((v) => received.push(v));

    mutable.set(1);
    expect(received).toEqual([1]);

    // Dispose multiple times
    sub.dispose();
    sub.dispose();

    mutable.set(2);
    expect(received).toEqual([1]);
  });
});
