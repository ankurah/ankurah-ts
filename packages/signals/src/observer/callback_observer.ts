// MIRRORS: ankurah/signals/src/observer/callback_observer.rs
import { Arc, Struct } from '@ankurah/base';
import type { Observer } from './index.ts';
import { CurrentObserver } from '../context.ts';
import { type Signal, ListenerGuard } from '../signal/index.ts';

// Auto-incrementing ID counter for observer identity
// Divergence: Rust uses Arc::as_ptr() as usize for observer_id;
// TS uses auto-incrementing counter since there are no pointer addresses [E8]
let nextObserverId = 0;

class SubscriptionEntry extends Struct {
  guard: ListenerGuard;
  markedForRemoval: boolean;

  constructor(guard: ListenerGuard, markedForRemoval: boolean) {
    super();
    this.guard = guard;
    this.markedForRemoval = markedForRemoval;
  }
}

class Inner extends Struct {
  // The callback to call when the observed signals notify the observer of a change
  // Divergence: Rust uses Box<dyn Fn() + Send + Sync>; TS uses plain function [E8]
  readonly callback: () => void;
  // Subscriptions mapped by broadcast ID for mark-and-sweep
  // Divergence: Rust uses RwLock<HashMap<BroadcastId, SubscriptionEntry>>;
  // TS uses plain Map since JS is single-threaded [E8].
  // Key is BroadcastId.toNumber() since class instances can't be Map keys by value.
  readonly entries: Map<number, SubscriptionEntry> = new Map();
  readonly observerId: number;

  constructor(callback: () => void) {
    super();
    this.callback = callback;
    this.observerId = nextObserverId++;
  }
}

/**
 * A CallbackObserver is an observer that wraps a callback which is called
 * whenever the observed signals notify the observer of a change.
 *
 * Rust: pub struct CallbackObserver(Arc<Inner>);
 * Divergence: Rust newtype around Arc<Inner>; TS class holding Arc<Inner> field [E8].
 */
export class CallbackObserver extends Struct implements Observer {
  private inner: Arc<Inner>;

  /** Create a new callback observer.
   * Rust: pub fn new<F: Fn() + Send + Sync + 'static>(callback: Arc<F>) -> Self
   * Divergence: Rust takes Arc<F>; TS takes plain function [E8].
   */
  constructor(callback: (() => void) | Arc<Inner>) {
    super();
    if (callback instanceof Arc) {
      // Internal: wrapping an existing Arc<Inner> (for clone/upgrade)
      this.inner = callback;
    } else {
      this.inner = Arc.new(new Inner(callback));
    }
  }

  /** Clone this observer (increments Arc refcount) */
  clone(): CallbackObserver {
    return new CallbackObserver(this.inner.clone());
  }

  /** Trigger the callback using this observer's context */
  trigger(): void {
    this.withContext(this.inner.value.callback);
  }

  /** Execute a function with this observer as the current context */
  withContext(f: () => void): void {
    // Mark all existing listeners for removal
    this.markAllForRemoval();

    CurrentObserver.set(this.clone());
    f();
    CurrentObserver.remove(this);

    // Sweep away any listeners that weren't preserved during the callback
    this.sweepMarkedListeners();
  }

  clear(): void {
    // Clear all listeners - they'll be dropped automatically
    const entries = this.inner.value.entries;
    for (const entry of entries.values()) {
      entry[Symbol.dispose]();
    }
    entries.clear();
  }

  /** Mark all existing listeners for removal (mark phase of mark-and-sweep) */
  private markAllForRemoval(): void {
    for (const entry of this.inner.value.entries.values()) {
      entry.markedForRemoval = true;
    }
  }

  /** Remove all listeners that are still marked for removal (sweep phase) */
  private sweepMarkedListeners(): void {
    const entries = this.inner.value.entries;
    for (const [key, entry] of entries) {
      if (entry.markedForRemoval) {
        entry[Symbol.dispose]();
        entries.delete(key);
      }
    }
  }

  // Observer trait implementation

  /** Observe a signal - subscribe to it if not already subscribed */
  observe(signal: Signal): void {
    // Use the signal's broadcast ID for identification
    const broadcastId = signal.broadcastId();
    const key = broadcastId.toNumber();

    const entries = this.inner.value.entries;

    // Check if we already have a listener for this broadcast
    const existing = entries.get(key);
    if (existing != null) {
      // We already have a Listener/ListenerGuard for this broadcast, just unmark it for removal
      existing.markedForRemoval = false;
      return;
    }

    // Create new listener
    // Rust: WeakCallbackObserver(Arc::downgrade(&self.0))
    const weak = this.inner.downgrade();
    const guard = signal.listen(() => {
      const upgraded = weak.upgrade();
      if (upgraded !== null) {
        const observer = new CallbackObserver(upgraded);
        observer.trigger();
      }
    });

    entries.set(key, new SubscriptionEntry(guard, false));
  }

  /** Get a unique identifier for this observer (for equality comparison) */
  observerId(): number {
    return this.inner.value.observerId;
  }
}
