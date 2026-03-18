// MIRRORS: ankurah/signals/src/observer.rs
// Exception E12: file-with-submodules pattern

import type { Signal } from '../signal/index.ts';

// Rust: mod callback_observer;
// Rust: pub use callback_observer::*;
export { CallbackObserver } from './callback_observer.ts';

// Divergence: Rust ObserverBounds trait (Send + Sync gating for multithread feature)
// is skipped entirely — JS is single-threaded [E8].

/**
 * An Observer is a struct that can observe multiple signals.
 *
 * Mirrors Rust `pub trait Observer: ObserverBounds`.
 *
 * Divergence: Rust has `as_any(&self) -> &dyn Any` for downcast support;
 * TS uses native `instanceof` instead, so this method is omitted [E8].
 */
export interface Observer {
  /** Observe a signal - implement this method to handle subscriptions */
  observe(signal: Signal): void;

  /**
   * Get a unique identifier for this observer (for equality comparison).
   * Rust: fn observer_id(&self) -> usize;
   */
  observerId(): number;
}
