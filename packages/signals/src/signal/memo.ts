// MIRRORS: ankurah/signals/src/signal/memo.rs
import { Struct } from '@ankurah/base';
import { CurrentObserver } from '../context.ts';
import { type BroadcastId } from '../broadcast.ts';
import { type Signal, type Listener, type Get, type Peek, type With, ListenerGuard } from './index.ts';
import { type Subscribe, SubscriptionGuard } from '../porcelain/index.ts';

/**
 * Like Map, but caches the transformed value until upstream notifies.
 * Useful when transform is expensive, or when output identity matters.
 *
 * Rust: pub struct Memo<Upstream, Input, Output, Transform>
 * Divergence: Rust uses four generic type params with trait bounds;
 * TS uses two (Input, Output) with interface constraints [E8].
 * Divergence: Rust uses Arc<RwLock<Option<Output>>> for cache;
 * TS uses plain mutable field since JS is single-threaded [E8].
 */
export class Memo<Input, Output> extends Struct implements Signal, Get<Output>, Peek<Output>, With<Output>, Subscribe<Output> {
  private source: Signal & With<Input>;
  private transform: (input: Input) => Output;
  /** Cached output - null means invalidated, needs recompute */
  private cached: Output | null = null;
  /** Keeps subscription to upstream alive - invalidates cache on notify */
  private _subscription: ListenerGuard;

  constructor(source: Signal & With<Input>, transform: (input: Input) => Output) {
    super();
    this.source = source;
    this.transform = transform;

    // Subscribe to upstream - invalidate cache on notify
    this._subscription = source.listen(() => {
      this.cached = null;
    });
  }

  /**
   * Ensure cache is populated, then call f with the cached value.
   * Rust: fn with_cached<R>(&self, f: impl FnOnce(&Output) -> R) -> R
   */
  private withCached<R>(f: (value: Output) => R): R {
    // Fast path: check if cached
    if (this.cached !== null) {
      return f(this.cached);
    }

    // Slow path: compute and cache
    // Divergence: Rust double-checks after acquiring write lock; not needed in single-threaded JS [E8]
    const output = this.source.with((input) => this.transform(input));
    this.cached = output;

    return f(this.cached!);
  }

  // Signal trait implementation — delegates to source

  /** Listen to changes to this signal with a listener function */
  listen(listener: Listener): ListenerGuard {
    return this.source.listen(listener);
  }

  /** Get the broadcast identifier for this signal */
  broadcastId(): BroadcastId {
    return this.source.broadcastId();
  }

  // With trait implementation

  /** Access the cached/computed value with a closure (tracked by CurrentObserver) */
  with<R>(f: (value: Output) => R): R {
    CurrentObserver.track(this.source);
    return this.withCached(f);
  }

  // Get trait implementation

  /** Get the current cached/computed value (tracked by CurrentObserver) */
  get(): Output {
    CurrentObserver.track(this.source);
    return this.withCached((v) => v);
  }

  // Peek trait implementation

  /** Get the current cached/computed value (NOT tracked by CurrentObserver) */
  peek(): Output {
    return this.withCached((v) => v);
  }

  // Subscribe trait implementation

  /** Subscribe to changes with a listener that receives the transformed value */
  subscribe(listener: (value: Output) => void): SubscriptionGuard {
    const source = this.source;
    const transform = this.transform;
    // Capture reference to this.cached for eager recompute in listener
    const memo = this;

    const guard = this.source.listen(() => {
      // Invalidate and recompute
      const output = source.with((input) => transform(input));
      memo.cached = output;
      listener(output);
    });

    return new SubscriptionGuard(guard);
  }
}
