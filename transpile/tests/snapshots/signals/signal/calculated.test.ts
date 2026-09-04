// MIRRORS: ankurah/signals/src/signal/calculated.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Calculated } from './calculated';
import { Arc } from '@ankurah/base';
import { Mut } from './mutable';

describe('calculated unit tests', () => {
  test('test_basic_calculated', () => {
    const a = Mut.new(1);
    const b = Mut.new(2);
    const sum = Calculated.new(((a, b) => {
      return () => a.get() + b.get();
    })(a.read(), b.read()));
    expect(sum.get()).toEqual(3);
    a.set(10);
    expect(sum.get()).toEqual(12);
    b.set(5);
    expect(sum.get()).toEqual(15);
  });

  test('test_two_independent_inputs', () => {
    const firstName = Mut.new('Alice');
    const lastName = Mut.new('Smith');
    const fullName = (() => {
      const first = firstName.read();
      const last = lastName.read();
      return Calculated.new(() => `${first.get()} ${last.get()}`);
    })();
    expect(fullName.get()).toEqual('Alice Smith');
    firstName.set('Bob');
    expect(fullName.get()).toEqual('Bob Smith');
    lastName.set('Jones');
    expect(fullName.get()).toEqual('Bob Jones');
    firstName.set('Carol');
    lastName.set('Williams');
    expect(fullName.get()).toEqual('Carol Williams');
  });

  test('test_calculated_with_closed_over_state', () => {
    const trigger = Mut.new(0);
    const counter = Calculated.new(((trigger) => {
      const count = Arc.new(0);
      return () => {
        const _ = trigger.get();
        return count.fetchAdd(1, undefined /* Ordering */.SeqCst) + 1;
      };
    })(trigger.read()));
    expect(counter.get()).toEqual(1);
    trigger.set(1);
    expect(counter.get()).toEqual(2);
    trigger.set(2);
    expect(counter.get()).toEqual(3);
  });

  test('test_calculated_downstream_subscription', () => {
    const source = Mut.new(5);
    const doubled = Calculated.new(((source) => {
      return () => source.get() * 2;
    })(source.read()));
    const callCount = Arc.new(0);
    const callCountRef = callCount.clone();
    const _sub = doubled.subscribe((value) => {
      expect(value).toEqual(20);
      callCountRef.fetchAdd(1, undefined /* Ordering */.SeqCst);
    });
    source.set(10);
    expect(callCount.load(undefined /* Ordering */.SeqCst)).toEqual(1);
  });

  test('test_chained_calculated', () => {
    const base = Mut.new(2);
    const doubled = Calculated.new(((base) => {
      return () => base.get() * 2;
    })(base.read()));
    const quadrupled = Calculated.new(() => doubled.get() * 2);
    expect(quadrupled.get()).toEqual(8);
    base.set(5);
    expect(quadrupled.get()).toEqual(20);
  });

  test('test_listener_does_not_pollute_dependencies', () => {
    const source = Mut.new(1);
    const unrelated = Mut.new(100);
    const computeCount = Arc.new(0);
    const computeCountRef = computeCount.clone();
    const doubled = Calculated.new(((source) => {
      return () => {
        computeCountRef.fetchAdd(1, undefined /* Ordering */.SeqCst);
        return source.get() * 2;
      };
    })(source.read()));
    expect(doubled.get()).toEqual(2);
    expect(computeCount.load(undefined /* Ordering */.SeqCst)).toEqual(1);
    const unrelatedRead = unrelated.read();
    const _sub = doubled.subscribe((_value) => {
      const _ = unrelatedRead.get();
    });
    source.set(2);
    expect(doubled.get()).toEqual(4);
    expect(computeCount.load(undefined /* Ordering */.SeqCst)).toEqual(2);
    unrelated.set(200);
    expect(doubled.get()).toEqual(4);
    expect(computeCount.load(undefined /* Ordering */.SeqCst)).toEqual(2);
  });

});
