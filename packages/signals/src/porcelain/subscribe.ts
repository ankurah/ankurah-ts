// MIRRORS: ankurah/signals/src/porcelain/subscribe.rs

import { Drop } from '@ankurah/base';
import type { ListenerGuard } from '../signal/index.ts';

// Rust: pub type SubscribeListener<T> = Box<dyn Fn(T) + Send + Sync + 'static>;
// Divergence: TS uses plain `(value: T) => void` callback, no Box needed [E8]

// Rust: pub trait IntoSubscribeListener<T>
// Divergence: Not needed in TS — closures are directly compatible as listeners [E8]

/**
 * Trait for subscribing to changes - provides the subscribe method.
 *
 * Rust: pub trait Subscribe<T: 'static>
 * In TS, we define the interface and implement it as methods on Mut/Read/Calculated/Map/Memo.
 */
export interface Subscribe<T> {
  /** Subscribe to changes with a listener that receives the new value */
  subscribe(listener: (value: T) => void): SubscriptionGuard;
}

// Rust: pub trait DynSubscribe<T: 'static>
// Divergence: DynSubscribe is a blanket impl over Subscribe in Rust (Box<dyn Fn> vs generic F).
// In TS there's no distinction — Subscribe covers both cases [E8].

// Rust: pub trait GetAndDynSubscribe<T: 'static>: Get<T> + Peek<T> + DynSubscribe<T>
// Divergence: Trait alias in Rust. Use intersection type at call sites in TS [E8].

/**
 * A guard for a subscription to a signal.
 * When disposed, the subscription is removed.
 *
 * Rust: pub struct SubscriptionGuard { _listenerguard: Box<dyn Any + Send + Sync> }
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

// Rust: IntoSubscribeListener impls for closures, std::sync::mpsc::Sender, tokio Sender
// Divergence: Not needed in TS — all listeners are plain functions [E8]
