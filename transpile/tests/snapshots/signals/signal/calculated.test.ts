// MIRRORS: ankurah/signals/src/signal/calculated.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Calculated, trigger } from './calculated';
import { Arc, OwnedClosure, checkedAdd, checkedMul, unsupported } from '@ankurah/base';
import { Mut } from './mutable';

describe('calculated unit tests', () => {
  test('test_basic_calculated', () => {
    const a = Mut.new(1);
    try {
      const b = Mut.new(2);
      try {
        const sum = Calculated.new(((a, b) => {
          return new OwnedClosure([a, b], () => checkedAdd(a.get(), b.get(), 'i32'));
        })(a.read(), b.read()));
        try {
          expect(sum.get()).toEqual(3);
          a.set(10);
          expect(sum.get()).toEqual(12);
          b.set(5);
          expect(sum.get()).toEqual(15);
        } finally {
          sum.drop();
        }
      } finally {
        b.drop();
      }
    } finally {
      a.drop();
    }
  });

  test('test_two_independent_inputs', () => {
    const firstName = Mut.new('Alice');
    try {
      const lastName = Mut.new('Smith');
      try {
        const fullName = (() => {
          const first = firstName.read();
          try {
            const last = lastName.read();
            try {
              return Calculated.new(() => `${first.get()} ${last.get()}`);
            } finally {
              last.drop();
            }
          } finally {
            first.drop();
          }
        })();
        try {
          expect(fullName.get()).toEqual('Alice Smith');
          firstName.set('Bob');
          expect(fullName.get()).toEqual('Bob Smith');
          lastName.set('Jones');
          expect(fullName.get()).toEqual('Bob Jones');
          firstName.set('Carol');
          lastName.set('Williams');
          expect(fullName.get()).toEqual('Carol Williams');
        } finally {
          fullName.drop();
        }
      } finally {
        lastName.drop();
      }
    } finally {
      firstName.drop();
    }
  });

  test('test_calculated_with_closed_over_state', () => {
    const trigger = Mut.new(0);
    try {
      const counter = Calculated.new(((trigger) => {
        const count = Arc.new(0);
        return new OwnedClosure([trigger, count], () => {
          const _ = trigger.get();
          return checkedAdd(unsupported('`fetch_add` WRITES what the `Arc<AtomicUsize>` holds, and it is reached through an accessor that hands out the value rather than the place'), 1, 'usize');
        });
      })(trigger.read()));
      try {
        expect(counter.get()).toEqual(1);
        trigger.set(1);
        expect(counter.get()).toEqual(2);
        trigger.set(2);
        expect(counter.get()).toEqual(3);
      } finally {
        counter.drop();
      }
    } finally {
      trigger.drop();
    }
  });

  test('test_calculated_downstream_subscription', () => {
    const source = Mut.new(5);
    try {
      const doubled = Calculated.new(((source) => {
        return new OwnedClosure([source], () => checkedMul(source.get(), 2, 'i32'));
      })(source.read()));
      try {
        const callCount = Arc.new(0);
        try {
          const callCountRef = callCount.clone();
          const _sub = doubled.subscribe(new OwnedClosure([callCountRef], (value: void) => {
            expect(value).toEqual(20);
            unsupported('`fetch_add` WRITES what the `Arc<AtomicUsize>` holds, and it is reached through an accessor that hands out the value rather than the place');
          }));
          try {
            source.set(10);
            expect(callCount.value).toEqual(1);
          } finally {
            _sub.drop();
          }
        } finally {
          callCount.drop();
        }
      } finally {
        doubled.drop();
      }
    } finally {
      source.drop();
    }
  });

  test('test_chained_calculated', () => {
    const base = Mut.new(2);
    try {
      const doubled = Calculated.new(((base) => {
        return new OwnedClosure([base], () => checkedMul(base.get(), 2, 'i32'));
      })(base.read()));
      const quadrupled = Calculated.new(new OwnedClosure([doubled], () => doubled.get() * 2));
      try {
        expect(quadrupled.get()).toEqual(8);
        base.set(5);
        expect(quadrupled.get()).toEqual(20);
      } finally {
        quadrupled.drop();
      }
    } finally {
      base.drop();
    }
  });

  test('test_listener_does_not_pollute_dependencies', () => {
    const source = Mut.new(1);
    try {
      const unrelated = Mut.new(100);
      try {
        const computeCount = Arc.new(0);
        try {
          const computeCountRef = computeCount.clone();
          const doubled = Calculated.new(((source) => {
            return new OwnedClosure([computeCountRef, source], () => {
              unsupported('`fetch_add` WRITES what the `Arc<AtomicUsize>` holds, and it is reached through an accessor that hands out the value rather than the place');
              return checkedMul(source.get(), 2, 'i32');
            });
          })(source.read()));
          try {
            expect(doubled.get()).toEqual(2);
            expect(computeCount.value).toEqual(1);
            const unrelatedRead = unrelated.read();
            const _sub = doubled.subscribe(new OwnedClosure([unrelatedRead], (_value: void) => {
              const _ = unrelatedRead.get();
            }));
            try {
              source.set(2);
              expect(doubled.get()).toEqual(4);
              expect(computeCount.value).toEqual(2);
              unrelated.set(200);
              expect(doubled.get()).toEqual(4);
              expect(computeCount.value).toEqual(2);
            } finally {
              _sub.drop();
            }
          } finally {
            doubled.drop();
          }
        } finally {
          computeCount.drop();
        }
      } finally {
        unrelated.drop();
      }
    } finally {
      source.drop();
    }
  });

});
