// MIRRORS: ankurah/signals/src/signal/read.rs

import { Broadcast, type BroadcastId, type BroadcastListener } from '../broadcast.ts';
import { ValueCell, type ReadValueCell } from '../value.ts';
import { ListenerGuard, type Signal, type Get, type Peek, type With, type GetReadCell, type Listener } from './index.ts';
import { SubscriptionGuard, type Subscribe } from '../porcelain/subscribe.ts';

/**
 * A read-only signal.
 *
 * Equivalent to Rust's `Read<T>`.
 * Created from a Mut<T> via .read().
 */
export class Read<T> implements Signal, Get<T>, Peek<T>, With<T>, GetReadCell<T>, Subscribe<T> {
  /** @internal - shares storage with the parent Mut<T> */
  private valueCell: ValueCell<T>;
  /** @internal - shares broadcast with the parent Mut<T> */
  private broadcast: Broadcast<void>;

  /** @internal */
  constructor(valueCell: ValueCell<T>, broadcast: Broadcast<void>) {
    this.valueCell = valueCell;
    this.broadcast = broadcast;
  }

  /**
   * Get the current value, tracked by the current context.
   * Phase 1: same as peek() (no observer tracking yet).
   */
  get(): T {
    // Phase 1: no CurrentObserver.track(this) yet
    return this.valueCell.getValue();
  }

  /** Get the current value without tracking */
  peek(): T {
    return this.valueCell.getValue();
  }

  /** Call a function with a reference to the current value */
  with<R>(f: (value: T) => R): R {
    // Phase 1: no CurrentObserver.track(this) yet
    return this.valueCell.with(f);
  }

  /** Get the read-only cell for this signal's value */
  getReadCell(): ReadValueCell<T> {
    return this.valueCell.readValue();
  }

  /** Listen to changes to this signal with a listener function */
  listen(listener: Listener): ListenerGuard {
    const broadcastListener: BroadcastListener<void> = {
      type: 'NotifyOnly',
      callback: listener,
    };
    const guard = this.broadcast.reference().listen(broadcastListener);
    return new ListenerGuard(guard);
  }

  /** Get the broadcast identifier for this signal */
  broadcastId(): BroadcastId {
    return this.broadcast.id();
  }

  /**
   * Subscribe to changes with a listener that receives the new value.
   * The listener is NOT called immediately, only when the signal changes.
   */
  subscribe(listener: (value: T) => void): SubscriptionGuard {
    const roValue = this.getReadCell();
    const guard = this.listen(() => {
      const currentValue = roValue.getValue();
      listener(currentValue);
    });
    return new SubscriptionGuard(guard);
  }
}
