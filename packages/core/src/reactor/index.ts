// MIRRORS: ankurah/core/src/reactor.rs
// Exception E12: file-with-submodules pattern

import type { CollectionId, EntityId, QueryId, Attested, Event } from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';
import { AsyncMutex } from '@ankurah/base';
import { Broadcast } from '@ankurah/signals';

import { Entity } from '../entity.ts';
import { EntityChange } from '../changes.ts';
import { SubscriptionError } from '../error.ts';
import { EntityResultSet } from '../resultset.ts';

import {
  Subscription,
  VecAccumulator,
  type ChangeNotification,
} from './subscription_state.ts';
import { ReactorSubscription } from './subscription.ts';
import {
  ReactorSubscriptionId,
  WatcherSet,
  type WatcherChange,
} from './watcherset.ts';
import { CandidateChanges } from './candidate_changes.ts';
import type { GapFetcher } from './fetch_gap.ts';

// ── Re-exports ────────────────────────────────────────────────────────

export { ComparisonIndex } from './comparison_index.ts';
export { PropertyPath } from './property_path.ts';
export { CandidateChanges } from './candidate_changes.ts';
export {
  ReactorSubscriptionId,
  WatcherSet,
  watcherChangeAdd,
  watcherChangeRemove,
  entityWatcherIdKey,
} from './watcherset.ts';
export type {
  WatcherOp,
  EntityWatcherId,
  WatcherChange,
  WatcherIdPair,
} from './watcherset.ts';
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
// Rust: pub trait PreNotifyHook { fn pre_notify(&self, version: u32); }
// Rust: impl PreNotifyHook for () { fn pre_notify(&self, _version: u32) {} }
// Divergence: Callback or null instead of trait; null = () no-op [E8].

export type PreNotifyHook = ((version: number) => void) | null;

// ── ReactorNodeLike ───────────────────────────────────────────────────
// Rust: trait TNodeErased<E> { async fn fetch_entities_from_local(...); ... }
// Divergence: Minimal interface to break circular import [E8].

export interface ReactorNodeLike {
  fetchEntitiesFromLocal(
    collectionId: CollectionId,
    selection: Selection,
  ): Promise<Entity[]>;
}

// ── EntityChangeNotification adapter ──────────────────────────────────
// Bridges EntityChange (properties) to ChangeNotification (methods).

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

// ── Reactor ───────────────────────────────────────────────────────────
// Rust: pub struct Reactor<E, Ev>(Arc<ReactorInner<E, Ev>>)
// Divergence: No generics (concrete Entity + Attested<Event>) [E8].
// Divergence: No Arc/Mutex — single-threaded JS [E8].

export class Reactor {
  // Rust: subscriptions: std::sync::Mutex<HashMap<ReactorSubscriptionId, Subscription<E, Ev>>>
  private subscriptions: Map<string, Subscription> = new Map();
  // Rust: watcher_set: Arc<std::sync::Mutex<WatcherSet>>
  private watcherSet: WatcherSet = new WatcherSet();
  // Rust: notify_lock: tokio::sync::Mutex<()>
  private notifyLock: AsyncMutex = new AsyncMutex();

  constructor() {}

  // Rust: pub fn subscribe(&self) -> ReactorSubscription<E, Ev>
  subscribe(): ReactorSubscription {
    const broadcast = new Broadcast<import('./update.ts').ReactorUpdate>();
    const subscription = new Subscription(broadcast, this.watcherSet);
    const subscriptionId = subscription.id();
    this.subscriptions.set(subscriptionId.toKey(), subscription);

    return new ReactorSubscription(
      subscriptionId,
      broadcast,
      {
        unsubscribe: (id) => this.unsubscribe(id),
        removeQuery: (subId, queryId) => this.removeQuery(subId, queryId),
        addEntitySubscriptions: (subId, entityIds) => this.addEntitySubscriptions(subId, entityIds),
        removeEntitySubscriptions: (subId, entityIds) => this.removeEntitySubscriptions(subId, entityIds),
      },
    );
  }

  // Rust: pub(crate) fn unsubscribe(&self, sub_id) -> Result<(), SubscriptionError>
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

  // Rust: pub fn remove_query(&self, subscription_id, query_id) -> Result<(), SubscriptionError>
  removeQuery(
    subscriptionId: ReactorSubscriptionId,
    queryId: QueryId,
  ): void {
    const subscription = this.subscriptions.get(subscriptionId.toKey());
    if (!subscription) {
      throw SubscriptionError.subscriptionNotFound();
    }

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

  // Rust: pub fn add_entity_subscriptions(&self, subscription_id, entity_ids)
  addEntitySubscriptions(
    subscriptionId: ReactorSubscriptionId,
    entityIds: Iterable<EntityId>,
  ): void {
    const subscription = this.subscriptions.get(subscriptionId.toKey());
    if (!subscription) return; // Rust: if let Some(subscription)

    for (const entityId of entityIds) {
      subscription.addEntitySubscription(entityId);
      this.watcherSet.addEntitySubscription(subscriptionId, entityId);
    }
  }

  // Rust: pub fn remove_entity_subscriptions(&self, subscription_id, entity_ids)
  removeEntitySubscriptions(
    subscriptionId: ReactorSubscriptionId,
    entityIds: Iterable<EntityId>,
  ): void {
    const subscription = this.subscriptions.get(subscriptionId.toKey());
    if (!subscription) return; // Rust: if let Some(subscription)

    for (const entityId of entityIds) {
      subscription.removeEntitySubscription(entityId);

      // TODO: Check if any predicates match this entity before removing from entity_watchers
      // For now, only remove if no predicates match
      const shouldRemove = !subscription.anyQueryMatches(entityId);
      if (shouldRemove) {
        this.watcherSet.removeEntitySubscription(subscriptionId, entityId);
      }
    }
  }

  // Rust: pub async fn add_query_and_notify<H: PreNotifyHook>(...)
  // Add a query and send initialization notification (for local subscriptions).
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
    // FIXME: Open question — is there a window where entity edits land between the local fetch
    // above and downstream notification handling (reactor.notify_change + evaluate_changes)
    // such that we need this gap fill to catch the missed edit-driven gap?
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

  // Rust: pub async fn update_query_and_notify<H: PreNotifyHook>(...)
  // Update an existing predicate (v>0) and send notifications.
  // Does diffing against the current resultset. Used by local LiveQuery updates.
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
    subscription.updateQuery(
      queryId,
      collectionId,
      selection,
      includedEntities,
      version,
      accumulator,
    );

    // Fill gaps if needed for this specific query
    // FIXME: Same open question as add_query_and_notify — do edits that slip in between the
    // storage fetch and subsequent notify_change path require this gap fill to keep limits tight?
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

  // Rust: pub async fn notify_change<C: ChangeNotification>(&self, changes: Vec<C>)
  // Divergence: Takes EntityChange[] directly and wraps to ChangeNotification [E8].
  async notifyChange(changes: EntityChange[]): Promise<void> {
    // Serialize notify_change invocations
    const release = await this.notifyLock.acquire();
    try {
      // Wrap changes as ChangeNotification
      const notifications: ChangeNotification[] = changes.map(
        (c) => new EntityChangeNotification(c),
      );

      // Phase 1: Build per-subscription candidate accumulators (first lock of watcher_set)
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

      // Phase 2: Parallelize evaluate_changes calls across subscriptions
      const evaluations: Promise<WatcherChange[]>[] = [];

      for (const [_subKey, { subscriptionId, candidates }] of candidatesBySub) {
        const subscription = this.subscriptions.get(subscriptionId.toKey());
        if (subscription) {
          evaluations.push(subscription.evaluateChanges(candidates));
        }
      }

      // Now await all evaluations (lock is dropped in Rust)
      const results = await Promise.all(evaluations);
      const allWatcherChanges: WatcherChange[] = results.flat();

      // Phase 3: Apply all watcher changes to watcher_set (second lock of watcher_set)
      for (const change of allWatcherChanges) {
        this.watcherSet.applyWatcherChange(change);
      }
    } finally {
      release();
    }
  }

  // Rust: pub fn system_reset(&self)
  // Notify all subscriptions that their entities have been removed but do not
  // remove the subscriptions.
  systemReset(): void {
    // Clear entity watchers first - no entities are being watched after reset,
    // because any previously existing entities "stopped existing" as part of the system reset.
    this.watcherSet.clearEntityWatchers();

    for (const subscription of this.subscriptions.values()) {
      subscription.systemReset();
    }
  }

  // NOTE: Rust has upsert_query on impl Reactor<Entity, Attested<Event>> for remote subscriptions
  // (server-side). Deferred to Layer 7.
}
