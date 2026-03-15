// MIRRORS: ankurah/core/src/reactor.rs
// Exception E12: file-with-submodules pattern

import type { CollectionId, EntityId, QueryId, Attested, Event } from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';
import { Broadcast } from '@ankurah/signals';

import { Entity } from '../entity.ts';
import { EntityChange } from '../changes.ts';
import { SubscriptionError } from '../error.ts';
import { EntityResultSet } from '../resultset.ts';

import {
  Subscription,
  VecAccumulator,
  type ChangeNotification,
  buildKeySpecFromSelection,
} from './subscription_state.ts';
import { ReactorSubscription } from './subscription.ts';
import {
  ReactorSubscriptionId,
  WatcherSet,
  type WatcherChange,
} from './watcher_set.ts';
import { CandidateChanges } from './candidate-changes.ts';
import type { GapFetcher } from './fetch_gap.ts';

// ── Re-exports ────────────────────────────────────────────────────────
// Re-export all sub-module types so consumers can import from reactor/index.

export { ComparisonIndex } from './comparison-index.ts';
export { PropertyPath } from './property-path.ts';
export { CandidateChanges } from './candidate-changes.ts';
export {
  ReactorSubscriptionId,
  WatcherSet,
  watcherChangeAdd,
  watcherChangeRemove,
  entityWatcherIdKey,
} from './watcher_set.ts';
export type {
  WatcherOp,
  EntityWatcherId,
  WatcherChange,
  WatcherIdPair,
} from './watcher_set.ts';
export { ReactorSubscription } from './subscription.ts';
export {
  Subscription,
  VecAccumulator,
  NoopAccumulator,
  buildKeySpecFromSelection,
} from './subscription_state.ts';
export type {
  ChangeNotification,
  UpdateItemAccumulator,
  QueryState,
} from './subscription_state.ts';
export type { GapFetcher } from './fetch_gap.ts';
export { QueryGapFetcher, buildContinuationPredicate, inferValueTypeForField } from './fetch_gap.ts';
export type { MembershipChange, ReactorUpdateItem, ReactorUpdate } from './update.ts';
export { hasMembershipChange } from './update.ts';

// ── PreNotifyHook ─────────────────────────────────────────────────────
// Mirrors Rust trait PreNotifyHook. The no-op case (Rust `()`) maps to null.

/**
 * Hook trait for performing actions before notification is sent.
 *
 * Rust: `pub trait PreNotifyHook { fn pre_notify(&self, version: u32); }`
 * Divergence: Callback or null instead of trait [E8].
 */
export type PreNotifyHook = ((version: number) => void) | null;

// ── NodeLike interface ────────────────────────────────────────────────
// Minimal interface for Node dependency to break circular imports.
// Mirrors Rust TNodeErased::fetch_entities_from_local.

/**
 * Minimal interface for the Node dependency used by Reactor.
 * Avoids circular import between Reactor and Node.
 *
 * Rust: `trait TNodeErased<E> { async fn fetch_entities_from_local(...); ... }`
 * Divergence: Only the method Reactor actually needs [E8].
 */
export interface ReactorNodeLike {
  fetchEntitiesFromLocal(
    collectionId: CollectionId,
    selection: Selection,
  ): Promise<Entity[]>;
}

// ── EntityChangeNotification adapter ──────────────────────────────────
// EntityChange has `entity` and `events` as properties, but ChangeNotification
// requires them as methods. This adapter bridges the gap.

class EntityChangeNotification implements ChangeNotification {
  private readonly _change: EntityChange;

  constructor(change: EntityChange) {
    this._change = change;
  }

  entity(): Entity {
    return this._change.entity;
  }

  events(): ReadonlyArray<Attested<Event>> {
    return this._change.events;
  }
}

import { AsyncMutex } from '@ankurah/base';

// ── Reactor ───────────────────────────────────────────────────────────
// Mirrors Rust struct Reactor<E, Ev>(Arc<ReactorInner<E, Ev>>).
// Divergence: No generics (concrete Entity + Attested<Event>) [E8].
// Divergence: No Arc — plain class instance (JS single-threaded) [E8].
// Divergence: No Mutex on subscriptions — plain Map [E8].

/**
 * A Reactor is a collection of subscriptions, which are to be notified
 * of changes to a set of entities.
 *
 * Rust: `pub struct Reactor<E, Ev>(Arc<ReactorInner<E, Ev>>)`
 * Divergence: No generics, Arc, or Mutex — single-threaded JS [E8].
 */
export class Reactor {
  /**
   * Active subscriptions keyed by ReactorSubscriptionId.toKey().
   * Rust: `std::sync::Mutex<HashMap<ReactorSubscriptionId, Subscription<E, Ev>>>`
   * Divergence: Plain Map (JS Maps preserve insertion order) [E8].
   */
  private subscriptions: Map<string, Subscription> = new Map();

  /**
   * Shared watcher routing table. Same object is passed to all Subscriptions.
   * Rust: `Arc<std::sync::Mutex<WatcherSet>>`
   * Divergence: Plain shared reference [E8].
   */
  private watcherSet: WatcherSet = new WatcherSet();

  /**
   * Serializes notifyChange invocations to ensure consistent watcher state.
   * Rust: `tokio::sync::Mutex<()>`
   * Rust: `tokio::sync::Mutex<()>` → AsyncMutex
   */
  private notifyLock: AsyncMutex = new AsyncMutex();

  constructor() {
    // All fields initialized in declarations.
  }

  // ── subscribe ─────────────────────────────────────────────────────

  /**
   * Create a new subscription container.
   *
   * Rust: `pub fn subscribe(&self) -> ReactorSubscription<E, Ev>`
   */
  subscribe(): ReactorSubscription {
    const broadcast = new Broadcast<import('./update.ts').ReactorUpdate>();
    const subscription = new Subscription(broadcast, this.watcherSet);
    const subscriptionId = subscription.id();
    this.subscriptions.set(subscriptionId.toKey(), subscription);

    return new ReactorSubscription(
      subscriptionId,
      broadcast,
      (id: ReactorSubscriptionId) => this.unsubscribe(id),
    );
  }

  // ── unsubscribe ───────────────────────────────────────────────────

  /**
   * Remove a subscription and all its predicates.
   *
   * Rust: `pub(crate) fn unsubscribe(&self, sub_id: ReactorSubscriptionId) -> Result<(), SubscriptionError>`
   */
  unsubscribe(subId: ReactorSubscriptionId): void {
    const subscription = this.subscriptions.get(subId.toKey());
    if (!subscription) {
      throw SubscriptionError.subscriptionNotFound();
    }
    this.subscriptions.delete(subId.toKey());

    // Get all queries for cleanup
    const queries = subscription.takeAllQueries();

    // Remove all predicates from watchers
    for (const [_queryIdStr, queryState] of queries) {
      // Remove from index watcher (only if selection was set)
      if (queryState.selection !== null) {
        this.watcherSet.recursePredicateWatchers(
          queryState.collectionId,
          queryState.selection.predicate,
          { subscriptionId: subId, queryId: queryState.queryId },
          'Remove',
        );
      }

      // Remove from entity watchers using predicate's matching entities
      const entityIds = queryState.resultset.keys();
      this.watcherSet.removeEntitySubscriptions(subId, entityIds);
    }
  }

  // ── removeQuery ───────────────────────────────────────────────────

  /**
   * Remove a predicate from a subscription.
   *
   * Rust: `pub fn remove_query(&self, subscription_id, query_id) -> Result<(), SubscriptionError>`
   */
  removeQuery(
    subscriptionId: ReactorSubscriptionId,
    queryId: QueryId,
  ): void {
    const subscription = this.subscriptions.get(subscriptionId.toKey());
    if (!subscription) {
      throw SubscriptionError.subscriptionNotFound();
    }

    // Remove the query from the subscription
    const queryState = subscription.removeQuery(queryId);
    if (!queryState) {
      throw SubscriptionError.predicateNotFound();
    }

    // Remove from watchers (only if selection was set)
    if (queryState.selection !== null) {
      const watcherId = { subscriptionId, queryId };
      this.watcherSet.recursePredicateWatchers(
        queryState.collectionId,
        queryState.selection.predicate,
        watcherId,
        'Remove',
      );
    }
  }

  // ── addEntitySubscriptions ────────────────────────────────────────

  /**
   * Add entity subscriptions to a subscription.
   *
   * Rust: `pub fn add_entity_subscriptions(&self, subscription_id, entity_ids)`
   */
  addEntitySubscriptions(
    subscriptionId: ReactorSubscriptionId,
    entityIds: Iterable<EntityId>,
  ): void {
    const subscription = this.subscriptions.get(subscriptionId.toKey());
    if (!subscription) return; // Silently ignore if not found (matches Rust behavior)

    for (const entityId of entityIds) {
      subscription.addEntitySubscription(entityId);
      this.watcherSet.addEntitySubscription(subscriptionId, entityId);
    }
  }

  // ── removeEntitySubscriptions ─────────────────────────────────────

  /**
   * Remove entity subscriptions from a subscription.
   *
   * Rust: `pub fn remove_entity_subscriptions(&self, subscription_id, entity_ids)`
   */
  removeEntitySubscriptions(
    subscriptionId: ReactorSubscriptionId,
    entityIds: Iterable<EntityId>,
  ): void {
    const subscription = this.subscriptions.get(subscriptionId.toKey());
    if (!subscription) return; // Silently ignore if not found (matches Rust behavior)

    for (const entityId of entityIds) {
      subscription.removeEntitySubscription(entityId);

      // Only remove from entity_watchers if no predicates still match
      const shouldRemove = !subscription.anyQueryMatches(entityId);
      if (shouldRemove) {
        this.watcherSet.removeEntitySubscription(subscriptionId, entityId);
      }
    }
  }

  // ── addQueryAndNotify ─────────────────────────────────────────────

  /**
   * Add a new query to a subscription (initial subscription only).
   * Fails if query_id already exists.
   *
   * Collects ReactorUpdateItems and sends them.
   * pre_notify_hook is called before sending notification.
   *
   * Rust: `pub async fn add_query_and_notify<H: PreNotifyHook>(...)`
   */
  async addQueryAndNotify(
    subscriptionId: ReactorSubscriptionId,
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    node: ReactorNodeLike,
    resultset: EntityResultSet,
    gapFetcher: GapFetcher,
    preNotifyHook: PreNotifyHook = null,
  ): Promise<void> {
    // Get subscription reference
    const subscription = this.subscriptions.get(subscriptionId.toKey());
    if (!subscription) {
      throw new Error(`Subscription ${subscriptionId} not found`);
    }

    // Fetch initial entities from local storage (do this first to avoid holding locks across await)
    const includedEntities = await node.fetchEntitiesFromLocal(collectionId, selection);

    // Register empty query state with subscription (will be populated by update_query)
    subscription.registerQuery(queryId, collectionId, resultset, gapFetcher);

    // Populate the resultset and collect ReactorUpdateItems
    // update_query now handles all watcher management internally (predicate + entity)
    const accumulator = new VecAccumulator();
    subscription.updateQuery(
      queryId,
      collectionId,
      selection,
      includedEntities,
      1, // version 1 for initial add
      accumulator,
    );

    // Fill gaps if needed for this specific query
    await subscription.fillGapsForQuery(queryId, accumulator);

    // Mark as loaded
    resultset.setLoaded(true);

    // Call pre-notify hook (e.g., mark LiveQuery as initialized) with version 1
    if (preNotifyHook !== null) {
      preNotifyHook(1);
    }

    // Send the notification with collected items. We always notify because we're initializing the query.
    subscription.sendUpdate(accumulator.items);
  }

  // ── updateQueryAndNotify ──────────────────────────────────────────

  /**
   * Update an existing query (v>0) and send notifications.
   * Does diffing against the current resultset.
   * Used by local LiveQuery updates.
   *
   * Rust: `pub async fn update_query_and_notify<H: PreNotifyHook>(...)`
   */
  async updateQueryAndNotify(
    subscriptionId: ReactorSubscriptionId,
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    node: ReactorNodeLike,
    version: number,
    preNotifyHook: PreNotifyHook = null,
  ): Promise<void> {
    const includedEntities = await node.fetchEntitiesFromLocal(collectionId, selection);

    const subscription = this.subscriptions.get(subscriptionId.toKey());
    if (!subscription) {
      throw new Error(`Subscription ${subscriptionId} not found`);
    }

    const accumulator = new VecAccumulator();
    // Update query - watcher management is handled internally
    subscription.updateQuery(
      queryId,
      collectionId,
      selection,
      includedEntities,
      version,
      accumulator,
    );

    // Fill gaps if needed for this specific query
    await subscription.fillGapsForQuery(queryId, accumulator);

    // Call pre-notify hook (e.g., mark LiveQuery as initialized)
    if (preNotifyHook !== null) {
      preNotifyHook(version);
    }

    // Send reactor update only if items is non-empty
    if (accumulator.items.length > 0) {
      subscription.sendUpdate(accumulator.items);
    }
  }

  // ── notifyChange ──────────────────────────────────────────────────

  /**
   * Notify subscriptions about entity changes.
   * Implements the three-phase notification pipeline:
   *   Phase 1: Accumulate interested watchers from WatcherSet
   *   Phase 2: Evaluate changes per subscription (CandidateChanges -> membership changes)
   *   Phase 3: Apply watcher mutations back to WatcherSet
   *
   * Rust: `pub async fn notify_change<C: ChangeNotification>(&self, changes: Vec<C>)`
   * Divergence: Takes EntityChange[] directly and wraps to ChangeNotification [E8].
   */
  async notifyChange(changes: EntityChange[]): Promise<void> {
    // Serialize notify_change invocations
    const release = await this.notifyLock.acquire();
    try {
      // Wrap changes as ChangeNotification for use with the generic pipeline
      const notifications: ChangeNotification[] = changes.map(
        (c) => new EntityChangeNotification(c),
      );

      // ── Phase 1: Accumulate interested watchers ──
      // Build per-subscription candidate accumulators
      const candidatesBySub = new Map<
        string,
        { subscriptionId: ReactorSubscriptionId; candidates: CandidateChanges<ChangeNotification> }
      >();

      for (let offset = 0; offset < notifications.length; offset++) {
        const change = notifications[offset];
        this.watcherSet.accumulateInterestedWatchers(
          change.entity(),
          offset,
          notifications,
          candidatesBySub,
        );
      }

      // ── Phase 2: Evaluate changes per subscription ──
      // Parallelize evaluate_changes calls across subscriptions (mirrors Rust join_all)
      const evaluations: Promise<WatcherChange[]>[] = [];

      for (const [_subKey, { subscriptionId, candidates }] of candidatesBySub) {
        const subscription = this.subscriptions.get(subscriptionId.toKey());
        if (subscription) {
          evaluations.push(subscription.evaluateChanges(candidates));
        }
      }

      // Await all evaluations
      const results = await Promise.all(evaluations);
      const allWatcherChanges: WatcherChange[] = results.flat();

      // ── Phase 3: Apply watcher changes to WatcherSet ──
      for (const change of allWatcherChanges) {
        this.watcherSet.applyWatcherChange(change);
      }
    } finally {
      release();
    }
  }

  // ── systemReset ───────────────────────────────────────────────────

  /**
   * Notify all subscriptions that their entities have been removed but do not
   * remove the subscriptions.
   *
   * Rust: `pub fn system_reset(&self)`
   */
  systemReset(): void {
    // Clear entity watchers first - no entities are being watched after reset,
    // because any previously existing entities "stopped existing" as part of the system reset.
    this.watcherSet.clearEntityWatchers();

    for (const subscription of this.subscriptions.values()) {
      subscription.systemReset();
    }
  }
}
