// MIRRORS: ankurah/signals/src/observer.rs
import { Arc } from '@ankurah/base';
export * from './observer/callback_observer';

export interface ObserverBounds {
}

export interface Observer {
  observe(signal: Signal): void;
  observerId(): number;
  asAny(): Any;
}

export function Observer_dispatch_observe(self: unknown, signal: Signal): void {
  if (self instanceof CallbackObserver) return (self as any).observe(signal);
  if (self instanceof Arc) return Arc_Inner_observe(self as any, signal);
  throw new Error(`BUG: no Observer impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

export function Observer_dispatch_observerId(self: unknown): number {
  if (self instanceof CallbackObserver) return (self as any).observerId();
  if (self instanceof Arc) return Arc_Inner_observerId(self as any);
  throw new Error(`BUG: no Observer impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

