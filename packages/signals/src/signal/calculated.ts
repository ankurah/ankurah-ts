// MIRRORS: ankurah/signals/src/signal/calculated.rs
import { Arc, Struct } from '@ankurah/base';
import type { Observer } from '../observer/index.ts';
import { CurrentObserver } from '../context.ts';
import { Broadcast, BroadcastId, type BroadcastListener } from '../broadcast.ts';
import { ValueCell, ReadValueCell } from '../value.ts';
import { type Signal, type Listener, type Get, type Peek, type With, type GetReadCell, ListenerGuard } from './index.ts';
import { type Subscribe, SubscriptionGuard } from '../porcelain/index.ts';

// Auto-incrementing ID for observer identity
// Divergence: Rust uses Arc::as_ptr() as usize; TS uses counter [E8]
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

class Inner<T> extends Struct {
  /** The compute function */
  // Divergence: Rust uses Box<dyn Fn() -> T + Send + Sync>; TS uses plain function [E8]
  readonly compute: () => T;
  /** Cached computed value */
  readonly value: ValueCell<T | null>;
  /** Broadcast for notifying downstream observers (fires AFTER context cleanup) */
  readonly broadcast: Broadcast<void>;
  /** Subscriptions to upstream signals, mapped by broadcast ID for mark-and-sweep */
  // Divergence: Rust uses RwLock<HashMap<BroadcastId, SubscriptionEntry>>;
  // TS uses plain Map since JS is single-threaded [E8].
  readonly entries: Map<number, SubscriptionEntry> = new Map();
  readonly observerId: number;

  constructor(compute: () => T) {
    super();
    this.compute = compute;
    this.value = new ValueCell<T | null>(null);
    this.broadcast = new Broadcast<void>();
    this.observerId = nextObserverId++;
  }
}

/**
 * Observer wrapper for Arc<Inner<T>>.
 * In Rust, Arc<Inner<T>> directly implements Observer.
 * In TS, we need a wrapper class that implements the Observer interface
 * and delegates to the Arc<Inner<T>> [E8].
 */
class InnerObserver<T> implements Observer {
  private arc: Arc<Inner<T>>;

  constructor(arc: Arc<Inner<T>>) {
    this.arc = arc;
  }

  observe(signal: Signal): void {
    const broadcastId = signal.broadcastId();
    const key = broadcastId.toNumber();
    const inner = this.arc.value;

    // Check if we already have a subscription for this signal
    const existing = inner.entries.get(key);
    if (existing != null) {
      existing.markedForRemoval = false;
      return;
    }

    // Create new subscription - when upstream changes, trigger recomputation
    const weak = this.arc.downgrade();
    const guard = signal.listen(() => {
      const upgraded = weak.upgrade();
      if (upgraded !== null) {
        trigger(upgraded);
        // Don't drop upgraded — trigger uses it, and it's a temporary strong ref
        upgraded.drop();
      }
    });

    inner.entries.set(key, new SubscriptionEntry(guard, false));
  }

  observerId(): number {
    return this.arc.value.observerId;
  }
}

/** Trigger recomputation with dependency tracking, then notify downstream */
function trigger<T>(arc: Arc<Inner<T>>): void {
  const inner = arc.value;

  // Mark-and-sweep: mark all existing subscriptions for removal
  for (const entry of inner.entries.values()) {
    entry.markedForRemoval = true;
  }

  // Set ourselves as the current observer and run compute
  const observer = new InnerObserver(arc);
  CurrentObserver.set(observer);
  const newValue = inner.compute();
  inner.value.set(newValue);
  CurrentObserver.remove(observer);

  // Sweep away any subscriptions that weren't accessed during compute
  for (const [key, entry] of inner.entries) {
    if (entry.markedForRemoval) {
      entry[Symbol.dispose]();
      inner.entries.delete(key);
    }
  }

  // NOW it's safe to broadcast - no observer context is active
  inner.broadcast.send(undefined as void);
}

/**
 * A calculated/derived signal that computes its value from other signals.
 *
 * Automatically tracks which signals are accessed during computation.
 * When any upstream signal changes, the computed value is recalculated
 * and downstream observers are notified.
 *
 * Rust: pub struct Calculated<T>(Arc<Inner<T>>);
 * Divergence: Rust newtype around Arc<Inner<T>>; TS class holding Arc<Inner<T>> [E8].
 */
export class Calculated<T> extends Struct implements Signal, Get<T>, Peek<T>, With<T>, GetReadCell<T | null>, Subscribe<T> {
  private inner: Arc<Inner<T>>;

  /**
   * Create a new calculated signal from a compute function.
   * The compute function will be called immediately to get the initial value,
   * and will be called again whenever any signal accessed during computation changes.
   */
  constructor(compute: (() => T) | Arc<Inner<T>>) {
    super();
    if (compute instanceof Arc) {
      // Internal: wrapping existing Arc<Inner<T>> (for clone)
      this.inner = compute;
    } else {
      this.inner = Arc.new(new Inner(compute));
      // Trigger initial computation to establish subscriptions and compute initial value
      trigger(this.inner);
    }
  }

  /** Clone this calculated signal (shares same underlying computed value and observer) */
  clone(): Calculated<T> {
    return new Calculated<T>(this.inner.clone());
  }

  // Get trait implementation

  /** Get the current value (tracked by CurrentObserver) */
  get(): T {
    CurrentObserver.track(this);
    return this.inner.value.value.with((opt) => {
      if (opt === null) throw new Error('Calculated value not initialized');
      return opt;
    });
  }

  // Peek trait implementation

  /** Get the current value (NOT tracked by CurrentObserver) */
  peek(): T {
    return this.inner.value.value.with((opt) => {
      if (opt === null) throw new Error('Calculated value not initialized');
      return opt;
    });
  }

  // With trait implementation

  /** Access the current value with a closure (tracked by CurrentObserver) */
  with<R>(f: (value: T) => R): R {
    CurrentObserver.track(this);
    return this.inner.value.value.with((opt) => {
      if (opt === null) throw new Error('Calculated value not initialized');
      return f(opt);
    });
  }

  // GetReadCell trait implementation

  /** Get a read-only cell containing the cached value */
  getReadCell(): ReadValueCell<T | null> {
    return this.inner.value.value.readvalue();
  }

  // Signal trait implementation

  /** Listen to changes to this signal with a listener function */
  listen(listener: Listener): ListenerGuard {
    const broadcastListener: BroadcastListener<void> = { type: 'NotifyOnly', callback: listener };
    return new ListenerGuard(this.inner.value.broadcast.reference().listen(broadcastListener));
  }

  /** Get the broadcast identifier for this signal */
  broadcastId(): BroadcastId {
    return this.inner.value.broadcast.id();
  }

  // Observer impl for Arc<Inner<T>> is in InnerObserver class above [E4]

  // Subscribe trait implementation

  /** Subscribe to changes with a listener that receives the new value */
  subscribe(listener: (value: T) => void): SubscriptionGuard {
    const roValue = this.inner.value.value.readvalue();
    const guard = this.listen(() => {
      const current = roValue.with((opt) => {
        if (opt === null) throw new Error('Calculated value not initialized');
        return opt;
      });
      listener(current);
    });
    return new SubscriptionGuard(guard);
  }
}
