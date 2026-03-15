// MIRRORS: ankurah/core/src/reactor/subscription.rs

import type { EntityId, QueryId } from '@ankurah/proto';
import {
  Broadcast,
  type BroadcastId,
  type BroadcastListener,
  type Signal,
  type Listener,
  ListenerGuard,
  type Subscribe,
  SubscriptionGuard,
} from '@ankurah/signals';
import { Drop } from '@ankurah/std';
import { SubscriptionError } from '../error.ts';
import { ReactorSubscriptionId } from './watcher_set.ts';
import type { ReactorUpdate } from './update.ts';

// ---------------------------------------------------------------------------
// ReactorSubInner
// ---------------------------------------------------------------------------

/**
 * Inner state for ReactorSubscription.
 *
 * Rust: `pub(super) struct ReactorSubInner<E, Ev>`
 *
 * Divergence: Rust stores an `Arc<ReactorSubInner>` and uses Drop to
 * call reactor.unsubscribe(). TS stores an unsubscribe callback instead
 * of a direct Reactor reference to avoid circular dependencies [E8].
 */
class ReactorSubInner {
  readonly subscriptionId: ReactorSubscriptionId;
  readonly broadcast: Broadcast<ReactorUpdate>;
  private readonly unsubscribeFn: (id: ReactorSubscriptionId) => void;
  private disposed = false;

  constructor(
    subscriptionId: ReactorSubscriptionId,
    broadcast: Broadcast<ReactorUpdate>,
    unsubscribeFn: (id: ReactorSubscriptionId) => void,
  ) {
    this.subscriptionId = subscriptionId;
    this.broadcast = broadcast;
    this.unsubscribeFn = unsubscribeFn;
  }

  /**
   * Mirrors Rust Drop for ReactorSubInner -- automatically unsubscribe from the reactor.
   * Divergence: JS has no Drop; callers must invoke drop() explicitly or via Symbol.dispose [E11].
   */
  drop(): void {
    if (!this.disposed) {
      this.disposed = true;
      this.unsubscribeFn(this.subscriptionId);
    }
  }
}

// ---------------------------------------------------------------------------
// ReactorSubscription
// ---------------------------------------------------------------------------

/**
 * A handle to a reactor subscription that automatically cleans up on drop.
 *
 * Rust: `pub struct ReactorSubscription<E, Ev>(Arc<ReactorSubInner<E, Ev>>)`
 *
 * Implements Signal (notify-only observation) and Subscribe<ReactorUpdate>
 * (payload observation), mirroring the Rust impl blocks.
 *
 * Divergence: Rust uses Arc for shared ownership; TS uses a simple reference
 * (single-threaded, no need for Arc). Clone is not needed since JS objects are
 * reference-counted by the GC [E8].
 *
 * Divergence: Rust generics E/Ev exist only for testing; TS uses concrete types [E7].
 */
export class ReactorSubscription extends Drop implements Signal, Subscribe<ReactorUpdate> {
  /** @internal */
  private readonly inner: ReactorSubInner;

  constructor(
    subscriptionId: ReactorSubscriptionId,
    broadcast: Broadcast<ReactorUpdate>,
    unsubscribeFn: (id: ReactorSubscriptionId) => void,
  ) {
    super('ReactorSubscription', 'fatal');
    this.inner = new ReactorSubInner(subscriptionId, broadcast, unsubscribeFn);
  }

  // ── Accessors ──────────────────────────────────────────────────────

  /** Get the subscription ID. Rust: `pub fn id(&self)` */
  id(): ReactorSubscriptionId {
    return this.inner.subscriptionId;
  }

  // ── Reactor delegation ─────────────────────────────────────────────

  // NOTE: Methods like remove_predicate, add_entity_subscriptions, and
  // remove_entity_subscriptions delegate to the Reactor in Rust. These
  // will be added once reactor/index.ts (the main Reactor) is ported,
  // as they require direct Reactor method calls.

  // ── Subscribe<ReactorUpdate> implementation ────────────────────────

  /**
   * Subscribe to ReactorUpdate notifications with a listener that receives the update payload.
   *
   * Rust: `impl Subscribe<ReactorUpdate<E, Ev>> for ReactorSubscription<E, Ev>`
   */
  subscribe(listener: (value: ReactorUpdate) => void): SubscriptionGuard {
    const broadcastListener: BroadcastListener<ReactorUpdate> = {
      type: 'Payload',
      callback: listener,
    };
    const guard = this.inner.broadcast.reference().listen(broadcastListener);
    return new SubscriptionGuard(new ListenerGuard(guard));
  }

  // ── Signal implementation ──────────────────────────────────────────

  /**
   * Listen to changes (notify-only, no payload).
   * This allows ReactorSubscription to be tracked by React observers
   * without cloning ReactorUpdate.
   *
   * Rust: `impl Signal for ReactorSubscription<E, Ev>`
   */
  listen(listener: Listener): ListenerGuard {
    const broadcastListener: BroadcastListener<ReactorUpdate> = {
      type: 'NotifyOnly',
      callback: () => listener(),
    };
    const guard = this.inner.broadcast.reference().listen(broadcastListener);
    return new ListenerGuard(guard);
  }

  /**
   * Get the broadcast identifier for this signal.
   *
   * Rust: `fn broadcast_id(&self) -> BroadcastId`
   */
  broadcastId(): BroadcastId {
    return this.inner.broadcast.id();
  }

  // ── Cleanup (mirrors Rust Drop) ────────────────────────────────────

  /**
   * Mirrors Rust's Drop impl on ReactorSubInner [E11].
   */
  protected onDrop(): void {
    this.inner.drop();
  }
}
