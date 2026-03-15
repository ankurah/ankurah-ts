// MIRRORS: ankurah/signals/src/signal.rs
// Exception E12: file-with-submodules pattern

import { Drop } from '@ankurah/std';
import { BroadcastId, type TListenerGuard } from '../broadcast.ts';

// Re-export submodules
export { Mut } from './mutable.ts';
export { Read } from './read.ts';
// Calculated is a stub (deferred)
export { } from './calculated.ts';

/**
 * Type alias for listener functions.
 * In Rust: Arc<dyn Fn(()) + Send + Sync + 'static>
 * In TS: just a callback [E8].
 */
export type Listener = () => void;

/**
 * Type-erased ListenerGuard that wraps any broadcast::ListenerGuard<T>.
 * In Rust, this uses Box<dyn TListenerGuard + Send + Sync>.
 * In TS, we just hold a reference to the inner guard.
 * Divergence: impl Drop -> extends Drop [E11].
 */
export class ListenerGuard extends Drop {
  private inner: TListenerGuard;

  /** Wrap any broadcast ListenerGuard<T> */
  constructor(guard: TListenerGuard) {
    super('ListenerGuard', 'warning');
    this.inner = guard;
  }

  /** Get the broadcast ID that this guard is subscribed to */
  broadcastId(): BroadcastId {
    return this.inner.broadcastId();
  }

  /** Unsubscribe (delegate to inner guard's drop) */
  protected onDrop(): void {
    if ('drop' in this.inner && typeof this.inner.drop === 'function') {
      this.inner.drop();
    }
  }
}

/**
 * Core trait for signals - provides observation capability without regard to a payload value.
 * The sole purpose of this trait is to provide a way to listen to changes to a signal.
 *
 * Note: Multiple signals may share the same broadcast (and thus the same broadcastId).
 * This is intentional and allows observers to deduplicate subscriptions efficiently.
 */
export interface Signal {
  /** Listen to changes to this signal with a listener function */
  listen(listener: Listener): ListenerGuard;

  /**
   * Get the broadcast identifier for this signal.
   * Multiple signals may return the same broadcastId if they share a broadcast.
   */
  broadcastId(): BroadcastId;
}

/**
 * Trait for getting the current value of a signal in a way that will be tracked
 * by the current context.
 *
 * Phase 1: get() behaves identically to peek() (no observer tracking yet).
 */
export interface Get<T> {
  get(): T;
}

/**
 * Trait for accessing the current value of a signal with a closure in a way that
 * will be tracked by the current context.
 *
 * Phase 1: with() does not track (no observer tracking yet).
 */
export interface With<T> {
  with<R>(f: (value: T) => R): R;
}

/**
 * Trait for getting the current value of a signal in a way that will NOT be tracked
 * by the current context.
 */
export interface Peek<T> {
  peek(): T;
}

/**
 * Trait for getting a read-only cell containing a present value.
 */
export interface GetReadCell<T> {
  getReadCell(): import('../value.ts').ReadValueCell<T>;
}
