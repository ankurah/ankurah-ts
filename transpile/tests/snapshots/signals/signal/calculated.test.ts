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
      const _ret = () => a.get() + b.get();
      b.drop();
      a.drop();
      return _ret;
    })(a.read(), b.read()));
    expect(sum.get()).toEqual(3);
    a.set(10);
    expect(sum.get()).toEqual(12);
    b.set(5);
    expect(sum.get()).toEqual(15);
    sum.drop();
    b.drop();
    a.drop();
  });

  test('test_two_independent_inputs', () => {
    const firstName = Mut.new('Alice'.toString());
    const lastName = Mut.new('Smith'.toString());
    const fullName = (() => {
      const first = firstName.read();
      const last = lastName.read();
      const _ret = Calculated.new(() => `${first.get()} ${last.get()}`);
      last.drop();
      first.drop();
      return _ret;
    })();
    expect(fullName.get()).toEqual('Alice Smith');
    firstName.set('Bob'.toString());
    expect(fullName.get()).toEqual('Bob Smith');
    lastName.set('Jones'.toString());
    expect(fullName.get()).toEqual('Bob Jones');
    firstName.set('Carol'.toString());
    lastName.set('Williams'.toString());
    expect(fullName.get()).toEqual('Carol Williams');
    fullName.drop();
    lastName.drop();
    firstName.drop();
  });

  test('test_calculated_with_closed_over_state', () => {
    const trigger = Mut.new(0);
    const counter = Calculated.new(((trigger) => {
      const count = Arc.new(0);
      const _ret = () => {
        const _ = trigger.get();
        return count.fetchAdd(1, undefined /* Ordering */.SeqCst) + 1;
      };
      count.drop();
      trigger.drop();
      return _ret;
    })(trigger.read()));
    expect(counter.get()).toEqual(1);
    trigger.set(1);
    expect(counter.get()).toEqual(2);
    trigger.set(2);
    expect(counter.get()).toEqual(3);
    counter.drop();
    trigger.drop();
  });

  test('test_calculated_downstream_subscription', () => {
    const source = Mut.new(5);
    const doubled = Calculated.new(((source) => {
      const _ret = () => source.get() * 2;
      source.drop();
      return _ret;
    })(source.read()));
    const callCount = Arc.new(0);
    const callCountRef = callCount.clone();
    const Sub = doubled.subscribe((value) => {
      expect(value).toEqual(20);
      callCountRef.fetchAdd(1, undefined /* Ordering */.SeqCst);
    });
    source.set(10);
    expect(callCount.load(undefined /* Ordering */.SeqCst)).toEqual(1);
    Sub.drop();
    callCountRef.drop();
    callCount.drop();
    doubled.drop();
    source.drop();
  });

  test('test_chained_calculated', () => {
    const base = Mut.new(2);
    const doubled = Calculated.new(((base) => {
      const _ret = () => base.get() * 2;
      base.drop();
      return _ret;
    })(base.read()));
    const quadrupled = Calculated.new(() => doubled.get() * 2);
    expect(quadrupled.get()).toEqual(8);
    base.set(5);
    expect(quadrupled.get()).toEqual(20);
    quadrupled.drop();
    doubled.drop();
    base.drop();
  });

  test('test_listener_does_not_pollute_dependencies', () => {
    const source = Mut.new(1);
    const unrelated = Mut.new(100);
    const computeCount = Arc.new(0);
    const computeCountRef = computeCount.clone();
    const doubled = Calculated.new(((source) => {
      const _ret = () => {
        computeCountRef.fetchAdd(1, undefined /* Ordering */.SeqCst);
        return source.get() * 2;
      };
      source.drop();
      return _ret;
    })(source.read()));
    expect(doubled.get()).toEqual(2);
    expect(computeCount.load(undefined /* Ordering */.SeqCst)).toEqual(1);
    const unrelatedRead = unrelated.read();
    const Sub = doubled.subscribe((Value) => {
      const _ = unrelatedRead.get();
    });
    source.set(2);
    expect(doubled.get()).toEqual(4);
    expect(computeCount.load(undefined /* Ordering */.SeqCst)).toEqual(2);
    unrelated.set(200);
    expect(doubled.get()).toEqual(4);
    expect(computeCount.load(undefined /* Ordering */.SeqCst)).toEqual(2);
    Sub.drop();
    unrelatedRead.drop();
    doubled.drop();
    computeCountRef.drop();
    computeCount.drop();
    unrelated.drop();
    source.drop();
  });

});
