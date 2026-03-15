// MIRRORS: ankurah/signals/src/broadcast.rs

import { Drop } from '@ankurah/std';

/**
 * A unique identifier for a broadcast that cannot be forged or extracted.
 * Can only be created by a Broadcast and used for deduplication/comparison.
 *
 * Uses auto-incrementing integer counter (not pointer-based as in Rust) [E8].
 */
let nextBroadcastId = 0;

export class BroadcastId {
  readonly value: number;

  /** @internal - only Broadcast should create BroadcastIds */
  constructor() {
    this.value = nextBroadcastId++;
  }

  equals(other: BroadcastId): boolean {
    return this.value === other.value;
  }

  toString(): string {
    return `${this.value}`;
  }
}

/**
 * A listener that can be called when broadcast notifications are sent.
 * Supports both full listeners (receive value) and unit listeners (notification only).
 */
export type BroadcastListener<T> =
  | { type: 'Payload'; callback: (value: T) => void }
  | { type: 'NotifyOnly'; callback: () => void };

/**
 * Trait for types that can be converted into broadcast listeners.
 * In TS, we implement this as overloaded listen() methods on BroadcastRef instead.
 */

/**
 * Trait for abstractly representing any broadcast ListenerGuard.
 */
export interface TListenerGuard {
  broadcastId(): BroadcastId;
}

/**
 * A subscription handle that can be used to unsubscribe from notifications.
 * Divergence: impl Drop -> extends Drop [E11].
 */
export class ListenerGuard<T = void> extends Drop implements TListenerGuard {
  private inner: Inner<T> | null;
  private id: number;
  private _broadcastId: BroadcastId;

  /** @internal */
  constructor(inner: Inner<T>, id: number, broadcastId: BroadcastId) {
    super('ListenerGuard', 'warning');
    this.inner = inner;
    this.id = id;
    this._broadcastId = broadcastId;
  }

  /** Get the broadcast ID that this guard is subscribed to */
  broadcastId(): BroadcastId {
    return this._broadcastId;
  }

  /** Unsubscribe from the broadcast (mirrors Rust's Drop) */
  protected onDrop(): void {
    if (this.inner !== null) {
      this.inner.listeners.delete(this.id);
      this.inner = null;
    }
  }
}

/**
 * Internal shared state for a Broadcast.
 * No Arc/RwLock needed - single-threaded JS [E8].
 */
class Inner<T> {
  listeners: Map<number, BroadcastListener<T>> = new Map();
  nextId: number = 0;
}

/**
 * A listen-only reference to a broadcast.
 */
export class BroadcastRef<T = void> {
  /** @internal */
  private inner: Inner<T>;
  /** @internal */
  private _broadcastId: BroadcastId;

  /** @internal */
  constructor(inner: Inner<T>, broadcastId: BroadcastId) {
    this.inner = inner;
    this._broadcastId = broadcastId;
  }

  /** Subscribe to notifications from the associated sender with a notification-only listener. */
  listen(listener: BroadcastListener<T>): ListenerGuard<T> {
    const id = this.inner.nextId++;
    this.inner.listeners.set(id, listener);
    return new ListenerGuard(this.inner, id, this._broadcastId);
  }

  /** Get a unique identifier for this broadcast (for deduplication purposes) */
  broadcastId(): BroadcastId {
    return this._broadcastId;
  }
}

/**
 * A broadcast sender that notifies multiple subscribers.
 * Uses synchronous function callbacks for immediate notification.
 *
 * No Arc needed - single-threaded JS [E8].
 */
export class Broadcast<T = void> {
  private _id: BroadcastId;
  private inner: Inner<T>;

  constructor() {
    this._id = new BroadcastId();
    this.inner = new Inner();
  }

  /** Get the unique identifier for this broadcast */
  id(): BroadcastId {
    return this._id;
  }

  /** Sends a notification to all active listeners */
  send(value: T): void {
    // Clone the listeners to avoid issues if listeners modify the map during iteration
    const subscribers = Array.from(this.inner.listeners.values());

    // Call all listeners
    for (const listener of subscribers) {
      switch (listener.type) {
        case 'Payload':
          listener.callback(value);
          break;
        case 'NotifyOnly':
          listener.callback();
          break;
      }
    }
  }

  /**
   * Get a read-only reference to this sender that can only subscribe to notifications.
   * This avoids exposing send() while still allowing subscription.
   */
  reference(): BroadcastRef<T> {
    return new BroadcastRef(this.inner, this._id);
  }
}
