// MIRRORS: ankurah/signals/src/signal/read.rs

import { Broadcast, type BroadcastId, type BroadcastListener } from '../broadcast.ts';
import { CurrentObserver } from '../context.ts';
import { ValueCell, type ReadValueCell } from '../value.ts';
import { ListenerGuard, type Signal, type Get, type Peek, type With, type GetReadCell, type Listener } from './index.ts';
import { SubscriptionGuard, type Subscribe } from '../porcelain/subscribe.ts';
import { Map } from './map.ts';
import { Memo } from './memo.ts';

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

  /** Returns a clone of the current value - not tracked by the current context */
  value(): T {
    return this.valueCell.getValue();
  }

  /** Create a mapped signal that transforms this signal's values on-demand */
  map<Output>(transform: (input: T) => Output): Map<T, Output> {
    return new Map<T, Output>(this, transform);
  }

  /** Create a memoized mapped signal - caches output until upstream changes */
  memo<Output>(transform: (input: T) => Output): Memo<T, Output> {
    return new Memo<T, Output>(this, transform);
  }

  /** Get the current value, tracked by the current context */
  get(): T {
    CurrentObserver.track(this);
    return this.valueCell.getValue();
  }

  /** Get the current value without tracking */
  peek(): T {
    return this.valueCell.getValue();
  }

  /** Call a function with a reference to the current value (tracked by CurrentObserver) */
  with<R>(f: (value: T) => R): R {
    CurrentObserver.track(this);
    return this.valueCell.with(f);
  }

  /** Get the read-only cell for this signal's value */
  getReadCell(): ReadValueCell<T> {
    return this.valueCell.readValue();
  }

  /**
   * Equality comparison - tracks signals used in the comparison.
   * Mirrors Rust: impl<T: PartialEq> PartialEq for Read<T>
   */
  equals(other: Read<T>): boolean {
    // Short-circuit if comparing to self
    if (this === other) {
      return true;
    }
    return this.with((selfVal) => other.with((otherVal) => selfVal === otherVal));
  }

  /** Display the current value as a string (tracked by CurrentObserver) */
  toString(): string {
    return this.with((v) => String(v));
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
