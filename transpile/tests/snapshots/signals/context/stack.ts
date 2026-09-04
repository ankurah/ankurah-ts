// MIRRORS: ankurah/signals/src/context/stack
import { Arc, RefCell, Ref, ThreadLocal } from '@ankurah/base';
import { Observer } from '../observer';
import { Signal } from '../signal';

export function track<S extends Signal>(signal: S): void {
  OBSERVER_STACK.with((stack) => {
    const _t0 = stack.borrow();
    try {
      {
        const _v = _t0.value.at(-1);
        if (_v != null) {
          const observer = _v;
          observer.value.observe(signal);
        }
      }
    } finally {
      _t0.drop();
    }
  });
}

export function set<O extends Observer>(observer: O): void {
  OBSERVER_STACK.with((stack) => {
    const _t0 = stack.borrowMut();
    try {
      _t0.value.push(Arc.new(observer));
    } finally {
      _t0.drop();
    }
  });
}

export function pop(): void {
  OBSERVER_STACK.with((stack) => {
    const _t0 = stack.borrowMut();
    try {
      _t0.value.pop();
    } finally {
      _t0.drop();
    }
  });
}

export function remove(observer: Observer): void {
  const targetId = observer.observerId();
  OBSERVER_STACK.with((stack) => {
    let stack_1 = stack.borrowMut();
    try {
      {
        const _v = stack_1.value.at(-1);
        if (_v != null) {
          const last = _v;
          if (last.value.observerId() === targetId) {
            stack_1.value.pop();
            return;
          }
        }
      }
      /* TODO: retain */ stack_1.value.filter((o) => o.observerId() !== targetId);
    } finally {
      stack_1.drop();
    }
  });
}

export function current(): Arc<Observer> | null {
  return OBSERVER_STACK.with((stack) => {
    const _t0 = stack.borrow();
    try {
      return _t0.value.at(-1);
    } finally {
      _t0.drop();
    }
  });
}

const OBSERVER_STACK = new ThreadLocal<RefCell<Arc<Observer>[]>>(new RefCell([]));

