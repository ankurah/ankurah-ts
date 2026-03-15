// MIRRORS: ankurah/signals/src/signal/calculated.rs
import { describe, test, expect } from 'bun:test';
import { Mut } from './mutable.ts';
import { Calculated } from './calculated.ts';

describe('Calculated', () => {
  // Rust: fn test_basic_calculated()
  test('basic calculated', () => {
    const a = new Mut(1);
    const b = new Mut(2);

    const sum = new Calculated(() => a.get() + b.get());

    expect(sum.get()).toBe(3);

    a.set(10);
    expect(sum.get()).toBe(12);

    b.set(5);
    expect(sum.get()).toBe(15);
  });

  // Rust: fn test_two_independent_inputs()
  test('two independent inputs', () => {
    const firstName = new Mut('Alice');
    const lastName = new Mut('Smith');

    const fullName = new Calculated(() => `${firstName.get()} ${lastName.get()}`);

    expect(fullName.get()).toBe('Alice Smith');

    // Change first name only
    firstName.set('Bob');
    expect(fullName.get()).toBe('Bob Smith');

    // Change last name only
    lastName.set('Jones');
    expect(fullName.get()).toBe('Bob Jones');

    // Change both
    firstName.set('Carol');
    lastName.set('Williams');
    expect(fullName.get()).toBe('Carol Williams');
  });

  // Rust: fn test_calculated_with_closed_over_state()
  test('calculated with closed-over state', () => {
    const trigger = new Mut(0);

    // Closed-over mutable state
    // Divergence: Rust uses Arc<AtomicUsize>; TS uses plain object [E8]
    const count = { value: 0 };

    const counter = new Calculated(() => {
      trigger.get(); // track the trigger
      count.value += 1;
      return count.value;
    });

    expect(counter.get()).toBe(1);

    trigger.set(1);
    expect(counter.get()).toBe(2);

    trigger.set(2);
    expect(counter.get()).toBe(3);
  });

  // Rust: fn test_calculated_downstream_subscription()
  test('calculated downstream subscription', () => {
    const source = new Mut(5);
    const doubled = new Calculated(() => source.get() * 2);

    let callCount = 0;

    const sub = doubled.subscribe((value) => {
      expect(value).toBe(20); // Should be 10 * 2
      callCount++;
    });

    source.set(10);

    expect(callCount).toBe(1);

    sub.drop();
  });

  // Rust: fn test_chained_calculated()
  test('chained calculated', () => {
    const base = new Mut(2);

    const doubled = new Calculated(() => base.get() * 2);
    const quadrupled = new Calculated(() => doubled.get() * 2);

    expect(quadrupled.get()).toBe(8);

    base.set(5);
    expect(quadrupled.get()).toBe(20);
  });

  // Rust: fn test_listener_does_not_pollute_dependencies()
  test('listener does not pollute dependencies', () => {
    const source = new Mut(1);
    const unrelated = new Mut(100);

    let computeCount = 0;

    const doubled = new Calculated(() => {
      computeCount++;
      return source.get() * 2;
    });

    expect(doubled.get()).toBe(2);
    expect(computeCount).toBe(1);

    // Subscribe a listener that reads an unrelated signal
    const sub = doubled.subscribe((_value) => {
      // This reads `unrelated` during the notification callback.
      // With a buggy implementation, this would cause `unrelated`
      // to be tracked as a dependency of `doubled`.
      unrelated.get();
    });

    // Change source - should trigger recompute
    source.set(2);
    expect(doubled.get()).toBe(4);
    expect(computeCount).toBe(2);

    // Change the unrelated signal - should NOT trigger recompute
    unrelated.set(200);
    expect(doubled.get()).toBe(4);
    expect(computeCount).toBe(2); // Still 2, no extra recompute

    sub.drop();
  });
});
