// MIRRORS: ankurah/signals/src/signal/mutable.rs

// Rust: use std::sync::Arc;
import { Broadcast, type BroadcastId, type BroadcastListener } from '../broadcast.ts';
import { CurrentObserver } from '../context.ts';
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

  // impl<T: 'static> Mut<T>

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

  /** Returns a read-only version of this signal */
  read(): Read<T> {
    return new Read(this.valueCell, this.broadcast);
  }

  // impl<T: Clone> Mut<T>

  /** Returns a clone of the current value - not tracked by the current context */
  value(): T {
    return this.valueCell.getValue();
  }

  // impl Get<T> for Mut<T>

  /** Get the current value, tracked by the current context */
  get(): T {
    CurrentObserver.track(this);
    return this.valueCell.getValue();
  }

  // impl Peek<T> for Mut<T>

  /** Get the current value without tracking */
  peek(): T {
    return this.valueCell.getValue();
  }

  // impl With<T> for Mut<T>

  /** Call a function with a reference to the current value (tracked by CurrentObserver) */
  with<R>(f: (value: T) => R): R {
    CurrentObserver.track(this);
    return this.valueCell.with(f);
  }

  // impl GetReadCell<T> for Mut<T>

  /** Get the read-only cell for this signal's value */
  getReadCell(): ReadValueCell<T> {
    return this.valueCell.readValue();
  }

  // impl Signal for Mut<T>

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

  // impl Subscribe<T> for Mut<T>

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
