// MIRRORS: ankurah/signals/src/context/stack
import { Arc, RefCell, Ref, ThreadLocal } from '@ankurah/base';
import { Observer } from '../observer';
import { Signal } from '../signal';

export function track<S extends Signal>(signal: S): void {
  OBSERVER_STACK.with((stack) => {
    if (stack.borrow().value.at(-1) != null) {
      const observer = stack.borrow().value.at(-1);
      observer.observe(signal);
    }
  });
}

export function set<O extends Observer>(observer: O): void {
  OBSERVER_STACK.with((stack) => {
    stack.borrowMut().value.push(Arc.new(observer));
  });
}

export function pop(): void {
  OBSERVER_STACK.with((stack) => {
    stack.borrowMut().value.pop();
  });
}

export function remove(observer: Observer): void {
  const targetId = observer.observerId();
  OBSERVER_STACK.with((stack) => {
    let stack_1 = stack.borrowMut();
    if (stack_1.value.at(-1) != null) {
      const last = stack_1.value.at(-1);
      if (last.observerId() === targetId) {
        stack_1.value.pop();
        return;

      }
    }
    /* TODO: retain */ stack_1.value.filter((o) => o.observerId() !== targetId);
    stack.drop();
  });
  targetId.drop();
}

export function current(): Arc<Observer> | null {
  return OBSERVER_STACK.with((stack) => [...stack.borrow().value.at(-1)]);
}

const OBSERVER_STACK = new ThreadLocal<RefCell<Arc<Observer>[]>>(new RefCell([]));

