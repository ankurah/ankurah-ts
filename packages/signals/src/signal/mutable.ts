// MIRRORS: ankurah/signals/src/signal/mutable.rs

import { Broadcast, type BroadcastId, type BroadcastListener } from '../broadcast.ts';
import { ValueCell, type ReadValueCell } from '../value.ts';
import { ListenerGuard, type Signal, type Get, type Peek, type With, type GetReadCell, type Listener } from './index.ts';
import { Read } from './read.ts';
import { SubscriptionGuard, type Subscribe } from '../porcelain/subscribe.ts';

/**
 * A mutable signal that can be read and written.
 *
 * Equivalent to Rust's `Mut<T>`.
 * No Clone derive needed in JS - objects are reference types.
 */
export class Mut<T> implements Signal, Get<T>, Peek<T>, With<T>, GetReadCell<T>, Subscribe<T> {
  private valueCell: ValueCell<T>;
  private broadcast: Broadcast<void>;

  constructor(value: T) {
    this.valueCell = new ValueCell(value);
    this.broadcast = new Broadcast<void>();
  }

  /** Update the value and notify all listeners */
  set(value: T): void {
    this.valueCell.set(value);
    // Notify all listeners
    this.broadcast.send(undefined as void);
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

  /** Returns a read-only version of this signal */
  read(): Read<T> {
    return new Read(this.valueCell, this.broadcast);
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
