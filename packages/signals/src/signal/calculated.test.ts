// MIRRORS: ankurah/signals/src/signal/calculated.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Calculated } from './calculated';
import { BincodeWriter, BincodeReader } from './codec';

describe('calculated unit tests', () => {
  test('test_basic_calculated', () => {
    const a = new Mut(1);
    const b = new Mut(2);
    const sum = new Calculated((() => {
      const a = a.read();
      const b = b.read();
      const _ret = () => a.get() + b.get();
      b.drop();
      a.drop();
      return _ret;
    })());
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
    const firstName = new Mut('Alice'.toString());
    const lastName = new Mut('Smith'.toString());
    const fullName = (() => {
      const first = firstName.read();
      const last = lastName.read();
      const _ret = new Calculated(() => `${first.get()} ${last.get()}`);
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
    const trigger = new Mut(0);
    const counter = new Calculated((() => {
      const trigger = trigger.read();
      const count = Arc.new(0);
      const _ret = () => (() => {
        const _ = trigger.get();
        return (() => { const _v = count; count += 1; return _v; })() + 1;
      })();
      count.drop();
      trigger.drop();
      return _ret;
    })());
    expect(counter.get()).toEqual(1);
    trigger.set(1);
    expect(counter.get()).toEqual(2);
    trigger.set(2);
    expect(counter.get()).toEqual(3);
    counter.drop();
    trigger.drop();
  });

  test('test_calculated_downstream_subscription', () => {
    const source = new Mut(5);
    const doubled = new Calculated((() => {
      const source = source.read();
      const _ret = () => source.get() * 2;
      source.drop();
      return _ret;
    })());
    const callCount = Arc.new(0);
    const callCountRef = callCount.clone();
    const Sub = doubled.subscribe((value) => (() => {
      expect(value).toEqual(20);
      (() => { const _v = callCountRef; callCountRef += 1; return _v; })();
    })());
    source.set(10);
    expect(callCount.load(Ordering.SeqCst)).toEqual(1);
    Sub.drop();
    callCountRef.drop();
    callCount.drop();
    doubled.drop();
    source.drop();
  });

  test('test_chained_calculated', () => {
    const base = new Mut(2);
    const doubled = new Calculated((() => {
      const base = base.read();
      const _ret = () => base.get() * 2;
      base.drop();
      return _ret;
    })());
    const quadrupled = new Calculated(() => doubled.get() * 2);
    expect(quadrupled.get()).toEqual(8);
    base.set(5);
    expect(quadrupled.get()).toEqual(20);
    quadrupled.drop();
    doubled.drop();
    base.drop();
  });

  test('test_listener_does_not_pollute_dependencies', () => {
    const source = new Mut(1);
    const unrelated = new Mut(100);
    const computeCount = Arc.new(0);
    const computeCountRef = computeCount.clone();
    const doubled = new Calculated((() => {
      const source = source.read();
      const _ret = () => (() => {
        (() => { const _v = computeCountRef; computeCountRef += 1; return _v; })();
        return source.get() * 2;
      })();
      source.drop();
      return _ret;
    })());
    expect(doubled.get()).toEqual(2);
    expect(computeCount.load(Ordering.SeqCst)).toEqual(1);
    const unrelatedRead = unrelated.read();
    const Sub = doubled.subscribe((Value) => (() => {
      const _ = unrelatedRead.get();
    })());
    source.set(2);
    expect(doubled.get()).toEqual(4);
    expect(computeCount.load(Ordering.SeqCst)).toEqual(2);
    unrelated.set(200);
    expect(doubled.get()).toEqual(4);
    expect(computeCount.load(Ordering.SeqCst)).toEqual(2);
    Sub.drop();
    unrelatedRead.drop();
    doubled.drop();
    computeCountRef.drop();
    computeCount.drop();
    unrelated.drop();
    source.drop();
  });

});
