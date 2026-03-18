// MIRRORS: ankurah/signals/src/signal/read.rs

// Rust: use std::sync::Arc;
import { Broadcast, type BroadcastId, type BroadcastListener } from '../broadcast.ts';
import { CurrentObserver } from '../context.ts';
import { ValueCell, type ReadValueCell } from '../value.ts';
import { ListenerGuard, type Signal, type Get, type Peek, type With, type GetReadCell, type Listener } from './index.ts';
import { SubscriptionGuard, type Subscribe } from '../porcelain/subscribe.ts';
import { Map } from './map.ts';
import { Memo } from './memo.ts';

/** Read-only signal */
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

  // impl<T: Clone> Read<T>

  /** Returns a clone of the current value - not tracked by the current context */
  value(): T {
    return this.valueCell.getValue();
  }

  // impl<T> Read<T>

  /** Create a mapped signal that transforms this signal's values on-demand */
  map<Output>(transform: (input: T) => Output): Map<T, Output> {
    return new Map<T, Output>(this, transform);
  }

  /** Create a memoized mapped signal - caches output until upstream changes */
  memo<Output>(transform: (input: T) => Output): Memo<T, Output> {
    return new Memo<T, Output>(this, transform);
  }

  // impl Get<T> for Read<T>

  /** Get the current value, tracked by the current context */
  get(): T {
    CurrentObserver.track(this);
    return this.valueCell.getValue();
  }

  // impl Peek<T> for Read<T>

  /** Get the current value without tracking */
  peek(): T {
    return this.valueCell.getValue();
  }

  // impl With<T> for Read<T>

  /** Call a function with a reference to the current value (tracked by CurrentObserver) */
  with<R>(f: (value: T) => R): R {
    CurrentObserver.track(this);
    return this.valueCell.with(f);
  }

  // impl GetReadCell<T> for Read<T>

  /** Get the read-only cell for this signal's value */
  getReadCell(): ReadValueCell<T> {
    return this.valueCell.readValue();
  }

  // impl Signal for Read<T>

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

  // impl PartialEq for Read<T>

  /**
   * Equality comparison - tracks signals used in the comparison.
   * Mirrors Rust: impl<T: PartialEq> PartialEq for Read<T>
   */
  equals(other: Read<T>): boolean {
    // Short-circuit if comparing to self to avoid deadlock from nested with calls
    if (this === other) {
      return true;
    }
    return this.with((selfVal) => other.with((otherVal) => selfVal === otherVal));
  }

  // impl Display for Read<T>

  /** Display the current value as a string (tracked by CurrentObserver) */
  toString(): string {
    return this.with((v) => String(v));
  }

  // impl Subscribe<T> for Read<T>

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
