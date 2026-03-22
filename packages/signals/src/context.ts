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

