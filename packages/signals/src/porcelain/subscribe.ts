// MIRRORS: ankurah/signals/src/porcelain/subscribe.rs

import type { ListenerGuard } from '../signal/index.ts';

/**
 * Trait for subscribing to changes - provides the subscribe method.
 *
 * In Rust, Subscribe<T> is a trait implemented on signal types.
 * In TS, we define the interface and implement it as methods on Mut/Read.
 */
export interface Subscribe<T> {
  /** Subscribe to changes with a listener that receives the new value */
  subscribe(listener: (value: T) => void): SubscriptionGuard;
}

/**
 * A guard for a subscription to a signal.
 * When disposed, the subscription is removed.
 *
 * In Rust this uses Box<dyn Any + Send + Sync> to type-erase the inner ListenerGuard.
 * In TS we just hold a reference to the ListenerGuard.
 */
export class SubscriptionGuard {
  private guard: ListenerGuard | null;

  constructor(guard: ListenerGuard) {
    this.guard = guard;
  }

  /** Unsubscribe (equivalent to Rust's Drop) */
  dispose(): void {
    if (this.guard !== null) {
      this.guard.dispose();
      this.guard = null;
    }
  }
}
