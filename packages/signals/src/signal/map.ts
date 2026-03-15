// MIRRORS: ankurah/signals/src/signal/map.rs
import { Struct } from '@ankurah/base';
import { CurrentObserver } from '../context.ts';
import { type BroadcastId } from '../broadcast.ts';
import { type Signal, type Listener, type Get, type Peek, type With, ListenerGuard } from './index.ts';
import { type Subscribe, SubscriptionGuard } from '../porcelain/index.ts';

/**
 * A signal that transforms values from another signal on-demand
 * without storing the transformed values.
 *
 * Rust: pub struct Map<Upstream, Input, Output, Transform>
 * Divergence: Rust uses four generic type params with trait bounds;
 * TS uses two type params (Input, Output) and interface constraints [E8].
 */
export class Map<Input, Output> extends Struct implements Signal, Get<Output>, Peek<Output>, With<Output>, Subscribe<Output> {
  private source: Signal & With<Input>;
  private transform: (input: Input) => Output;

  constructor(source: Signal & With<Input>, transform: (input: Input) => Output) {
    super();
    this.source = source;
    this.transform = transform;
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

  /** Access the transformed value with a closure (tracked by CurrentObserver) */
  with<R>(f: (value: Output) => R): R {
    // Track the source signal with the current observer
    CurrentObserver.track(this.source);
    // Get the source value and transform it on-demand
    return this.source.with((input) => {
      const output = this.transform(input);
      return f(output);
    });
  }

  // Get trait implementation

  /** Get the current transformed value (tracked by CurrentObserver) */
  get(): Output {
    // Track the source signal with the current observer
    CurrentObserver.track(this.source);
    // Get the source value and transform it on-demand, returning owned value
    return this.source.with((input) => this.transform(input));
  }

  // Peek trait implementation

  /** Get the current transformed value (NOT tracked by CurrentObserver) */
  peek(): Output {
    // Get the source value and transform it on-demand, returning owned value
    return this.source.with((input) => this.transform(input));
  }

  // Subscribe trait implementation

  /** Subscribe to changes with a listener that receives the transformed value */
  subscribe(listener: (value: Output) => void): SubscriptionGuard {
    const source = this.source;
    const transform = this.transform;

    const guard = this.source.listen(() => {
      source.with((input) => {
        listener(transform(input));
      });
    });

    return new SubscriptionGuard(guard);
  }
}
