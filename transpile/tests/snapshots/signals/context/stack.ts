// MIRRORS: ankurah/signals/src/context/stack
import { Arc, RefCell, ThreadLocal, invokeRef, Invocable, dropOwned } from '@ankurah/base';
import { Observer, Observer_dispatch_observe, Observer_dispatch_observerId } from '../observer';
import { Signal } from '../signal';

export function track<S extends Signal>(signal: S): void {
  OBSERVER_STACK.with((stack) => {
    const _t0 = stack.borrow();
    try {
      {
        const _v = _t0.value.at(-1);
        if (_v != null) {
          const observer = _v;
          Observer_dispatch_observe(observer.value, signal);
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
      dropOwned(_t0.value.pop());
    } finally {
      _t0.drop();
    }
  });
}

export function remove(observer: Observer): void {
  const targetId = Observer_dispatch_observerId(observer);
  OBSERVER_STACK.with((stack) => {
    let stack_1 = stack.borrowMut();
    try {
      {
        const _v = stack_1.value.at(-1);
        if (_v != null) {
          const last = _v;
          if (Observer_dispatch_observerId(last.value) === targetId) {
            dropOwned(stack_1.value.pop());
            return;
          }
        }
      }
      ((<T,>($xs: T[], $p: Invocable<[T], boolean>) => {
        let $at = 0;
        let $i = 0;
        try {
          for (; $i < $xs.length; $i++) {
            if (invokeRef($p, $xs[$i])) { $xs[$at++] = $xs[$i]; } else { dropOwned($xs[$i]); }
          }
        } finally {
          for (; $i < $xs.length; $i++) $xs[$at++] = $xs[$i];
          $xs.length = $at;
          dropOwned($p);
        }
      })(stack_1.value, (o) => Observer_dispatch_observerId(o.value) !== targetId));
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

