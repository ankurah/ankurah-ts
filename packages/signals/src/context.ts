// MIRRORS: ankurah/signals/src/context.rs
import { Struct, Arc } from '@ankurah/base';

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
  OBSERVER_STACK.with((stack) => if (stack.borrow().at(-1) != null) {
    const observer = stack.borrow().at(-1);
    observer.observe(signal);
  });
}

export function set(observer: O): void {
  OBSERVER_STACK.with((stack) => (() => {
    stack.borrowMut().push(Arc.new(observer));
  })());
}

export function pop(): void {
  OBSERVER_STACK.with((stack) => (() => {
    stack.borrowMut().pop();
  })());
}

export function remove(observer: Observer): void {
  const targetId = observer.observerId();
  OBSERVER_STACK.with((stack) => (() => {
    let stack = stack.borrowMut();
    if (/* let last = stack.at(-1) */ && last.observerId() === targetId) {
      stack.pop();
      return;
    }
    stack.retain((o) => o.observerId() !== targetId);
    stack.drop();
  })());
  targetId.drop();
}

export function current(): Arc<Observer> | null {
  return OBSERVER_STACK.with((stack) => [...stack.borrow().at(-1)]);
}

