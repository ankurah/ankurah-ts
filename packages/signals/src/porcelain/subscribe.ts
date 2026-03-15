// MIRRORS: ankurah/signals/src/porcelain/subscribe.rs

import { Drop } from '@ankurah/base';
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
 * Divergence: impl Drop -> extends Drop [E11].
 */
export class SubscriptionGuard extends Drop {
  private guard: ListenerGuard | null;

  constructor(guard: ListenerGuard) {
    super();
    this.guard = guard;
  }

  /** Unsubscribe (mirrors Rust's Drop) */
  drop(): void {
    if (this.guard !== null) {
      this.guard.drop();
      this.guard = null;
    }
  }
}
