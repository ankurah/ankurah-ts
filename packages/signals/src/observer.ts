// MIRRORS: ankurah/signals/src/observer.rs

export interface ObserverBounds {
}

export interface ObserverBounds {
}

export interface Observer {
  observe(signal: Signal): void;
  observerId(): number;
  asAny(): Any;
}

