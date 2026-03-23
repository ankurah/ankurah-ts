// MIRRORS: ankurah/signals/src/context.rs
import { Struct, Arc, RefCell, Ref, ThreadLocal } from '@ankurah/base';
import { Observer } from './observer';

export class CurrentObserver extends Struct {

  static track<S>(): void {
    stack.track(signal);
  }

  static set<O extends Observer>(observer: O): void {
    stack.set(observer);
  }

  static pop(): void {
    stack.pop();
  }

  static remove(observer: Observer): void {
    stack.remove(observer);
  }

  static current(): Arc<Observer> | null {
    return stack.current();
  }
}

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
    stack = stack.borrowMut();
    if (stack.last() != null) {
      const last = stack.last();
      if (last.observerId() === targetId) {
        stack.pop();
        return;

      }
    }
    stack.retain((o) => o.observerId() !== targetId);
    stack.drop();
  });
  targetId.drop();
}

export function current(): Arc<Observer> | null {
  return OBSERVER_STACK.with((stack) => [...stack.borrow().last()]);
}

const OBSERVER_STACK = new ThreadLocal<RefCell<Arc<Observer>[]>>(new RefCell([]));

