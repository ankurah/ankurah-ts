// MIRRORS: ankurah/signals/src/context/stack
import { Arc, RefCell, Ref, ThreadLocal } from '@ankurah/base';
import { Observer } from '../observer';

export function track(signal: S): void {
  OBSERVER_STACK.with((stack) => {
    if (stack.borrow().last() != null) {
      const observer = stack.borrow().last();
      observer.observe(signal);
    }
  });
}

export function set(observer: O): void {
  OBSERVER_STACK.with((stack) => {
    stack.borrowMut().push(Arc.new(observer));
  });
}

export function pop(): void {
  OBSERVER_STACK.with((stack) => {
    stack.borrowMut().pop();
  });
}

export function remove(observer: Observer): void {
  const targetId = observer.observerId();
  OBSERVER_STACK.with((stack) => {
    if (stack.last() != null) {
      const last = stack.last();
      if (last.observerId() === targetId) {
        stack.pop();
        return;

      }
    }
    { for (const [_k, _v] of stack) { if (!((o) => o.observerId() !== targetId(_k, _v))) stack.delete(_k); } };
    stack.drop();
  });
  targetId.drop();
}

export function current(): Arc<Observer> | null {
  return OBSERVER_STACK.with((stack) => [...stack.borrow().last()]);
}

const OBSERVER_STACK = new ThreadLocal<RefCell<Arc<Observer>[]>>(new RefCell([]));

