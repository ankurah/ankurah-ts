// MIRRORS: ankurah/signals/src/context.rs
import { Struct, Arc } from '@ankurah/base';
import { Observer } from './observer';
import { current, pop, remove, set, track } from './context/stack';

export class CurrentObserver extends Struct {

  static track<S>(): void {
    track(signal);
  }

  static set<O extends Observer>(observer: O): void {
    set(observer);
  }

  static pop(): void {
    pop();
  }

  static remove(observer: Observer): void {
    remove(observer);
  }

  static current(): Arc<Observer> | null {
    return current();
  }
}

