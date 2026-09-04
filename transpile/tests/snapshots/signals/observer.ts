// MIRRORS: ankurah/signals/src/observer.rs
export * from './observer/callback_observer';

export interface ObserverBounds {
}

export interface Observer {
  observe(signal: Signal): void;
  observerId(): number;
  asAny(): Any;
}

