// MIRRORS: ankurah/signals/src/broadcast.rs
import { Struct, Drop, Arc, Weak } from '@ankurah/base';

/** A unique identifier for a broadcast that cannot be forged or extracted.
 * Can only be created by a Broadcast and used for deduplication/comparison. */
// Divergence: Rust uses pointer-based usize ID; TS uses auto-incrementing counter [E8]
let nextBroadcastIdCounter = 0;

export class BroadcastId extends Struct {
  private readonly inner: number;

  /** @internal - only Broadcast should create BroadcastIds */
  constructor(id?: number) {
    super();
    this.inner = id ?? nextBroadcastIdCounter++;
  }

  toNumber(): number {
    return this.inner;
  }

  equals(other: BroadcastId): boolean {
    return this.inner === other.inner;
  }

  toString(): string {
    return `${this.inner}`;
  }

  clone(): BroadcastId {
    return new BroadcastId(this.inner);
  }
}

/** A listener that can be called when broadcast notifications are sent.
 * Supports both full listeners (receive value) and unit listeners (notification only). */
// Divergence: Rust enum with Arc<dyn Fn> variants; TS uses discriminated union type
// since callers construct these inline and Enum<V> would break them [E8]
export type BroadcastListener<T = void> =
  | { type: 'Payload'; callback: (value: T) => void }
  | { type: 'NotifyOnly'; callback: () => void };

// Trait for types that can be converted into broadcast listeners.
// In TS, implemented as overloaded listen() methods on BroadcastRef instead.

/** Trait for abstractly representing any ListenerGuard<T> */
export interface TListenerGuard {
  broadcastId(): BroadcastId;
}

// Internal shared state for a Broadcast.
class Inner<T> extends Struct {
  listeners: Map<number, BroadcastListener<T>> = new Map();
  nextId: number = 0;
}

/** A subscription handle that can be used to unsubscribe from notifications.
 * impl Drop -> extends Drop [E11] */
export class ListenerGuard<T = void> extends Drop implements TListenerGuard {
  private inner: Weak<Inner<T>>;
  private id: number;
  private _broadcastId: BroadcastId;

  /** @internal */
  constructor(inner: Weak<Inner<T>>, id: number, broadcastId: BroadcastId) {
    super();
    this.inner = inner;
    this.id = id;
    this._broadcastId = broadcastId;
  }

  /** Get the broadcast ID that this guard is subscribed to */
  broadcastId(): BroadcastId {
    // A ListenerGuard does not keep the broadcast alive
    // but the address is reserved until all Arc/Weak references are dropped
    // Given that we are using the address as the ID, this is safe.
    // We don't actually care if the broadcast is alive. The point is to
    // provide a unique id for removing the correct listener.
    return this._broadcastId;
  }

  /** Automatically unsubscribes when the subscription handle is dropped. */
  drop(): void {
    const upgraded = this.inner.upgrade();
    if (upgraded !== null) {
      upgraded.value.listeners.delete(this.id);
      upgraded.drop();
    }
    this.inner.drop();
  }
}

/** A listen-only reference to a broadcast */
// Divergence: Rust Ref<'a, T> uses a borrow of Broadcast; TS holds Arc clone [E8]
export class BroadcastRef<T = void> extends Struct {
  /** @internal */
  private arc: Arc<Inner<T>>;
  /** @internal */
  private _broadcastId: BroadcastId;

  /** @internal */
  constructor(arc: Arc<Inner<T>>, broadcastId: BroadcastId) {
    super();
    this.arc = arc;
    this._broadcastId = broadcastId;
  }

  /** Subscribe to notifications from the associated sender. */
  listen(listener: BroadcastListener<T>): ListenerGuard<T> {
    const id = this.arc.value.nextId++;
    this.arc.value.listeners.set(id, listener);
    return new ListenerGuard(this.arc.downgrade(), id, this._broadcastId);
  }

  /** Get a unique identifier for this broadcast (for deduplication purposes) */
  broadcastId(): BroadcastId {
    return this._broadcastId;
  }
}

/** A broadcast sender that notifies multiple subscribers.
 * Uses synchronous function callbacks for immediate notification. */
export class Broadcast<T = void> extends Struct {
  private arc: Arc<Inner<T>>;
  private _id: BroadcastId;

  constructor(arc?: Arc<Inner<T>>, id?: BroadcastId) {
    super();
    this.arc = arc ?? Arc.new(new Inner());
    this._id = id ?? new BroadcastId();
  }

  clone(): Broadcast<T> {
    return new Broadcast<T>(this.arc.clone(), this._id.clone());
  }

  /** Get the unique identifier for this broadcast */
  id(): BroadcastId {
    return this._id;
  }

  /** Sends a notification to all active listeners */
  send(value: T): void {
    // Clone the listeners to avoid holding the lock during callback execution
    // maybe someday we can avoid the alloc here using a thread-local buffer?
    const subscribers = Array.from(this.arc.value.listeners.values());

    // Call all listeners without holding any locks
    // clone the value for each subscriber except the last one
    if (subscribers.length > 0) {
      const last = subscribers[subscribers.length - 1];
      const rest = subscribers.slice(0, -1);

      for (const callback of rest) {
        switch (callback.type) {
          case 'Payload':
            callback.callback(value);
            break;
          case 'NotifyOnly':
            callback.callback();
            break;
        }
      }
      switch (last.type) {
        case 'Payload':
          last.callback(value);
          break;
        case 'NotifyOnly':
          last.callback();
          break;
      }
    }
  }

  /**
   * Get a read-only reference to this sender that can only subscribe to notifications.
   * This avoids cloning the sender while still forbidding the user from sending notifications.
   */
  reference(): BroadcastRef<T> {
    return new BroadcastRef(this.arc.clone(), this._id);
  }
}
