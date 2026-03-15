// MIRRORS: ankurah/core/src/reactor/subscription_state.rs

import type { CollectionId, EntityId, QueryId, Attested, Event } from '@ankurah/proto';
import type { Selection, Predicate, OrderByItem } from '@ankurah/ankql';
import { Broadcast } from '@ankurah/signals';

import { Entity } from '../entity.ts';
import { EntityResultSet } from '../resultset.ts';
import { evaluatePredicate, type Filterable } from '../selection/filter.ts';
import type { Value } from '../value/index.ts';
import { ValueType, valueType } from '../value/index.ts';
import { IndexDirection, NullsOrder, type IndexKeyPart, type KeySpec } from '../indexing/index.ts';
import type { GapFetcher } from './fetch_gap.ts';
import { CandidateChanges } from './candidate_changes.ts';
import type { WatcherChange } from './watcherset.ts';
import {
  ReactorSubscriptionId,
  WatcherSet,
  watcherChangeAdd,
  watcherChangeRemove,
  type WatcherIdPair,
} from './watcherset.ts';
import type { MembershipChange, ReactorUpdate, ReactorUpdateItem } from './update.ts';

// ── ChangeNotification interface ──────────────────────────────────────
// Mirrors Rust trait ChangeNotification in reactor.rs.
// The Rust version is generic (Entity = E, Event = Ev); TS uses concrete types [E8].

/**
 * Trait for types that can be used in notify_change.
 *
 * Rust: `pub trait ChangeNotification { type Entity; type Event; ... }`
 * Divergence: TS uses concrete Entity and Attested<Event> types [E8].
 */
export interface ChangeNotification {
  entity(): Entity;
  events(): ReadonlyArray<Attested<Event>>;
}

// ── Entity → Filterable adapter ───────────────────────────────────────
// Entity does not directly implement Filterable (collection() returns CollectionId, not string;
// method is getPropertyValue, not value). This adapter bridges the gap.

function entityAsFilterable(entity: Entity): Filterable {
  return {
    collection(): string {
      return entity.collectionId.toString();
    },
    value(name: string): Value | null {
      return entity.getPropertyValue(name);
    },
  };
}

// ── UpdateItemAccumulator ─────────────────────────────────────────────
// Mirrors Rust trait UpdateItemAccumulator<E, Ev>
// Allows for both collecting items (array) and discarding them (no-op).

/**
 * Accumulates ReactorUpdateItems during update_query.
 * Rust: `pub trait UpdateItemAccumulator<E, Ev>`
 */
export interface UpdateItemAccumulator {
  pushInitial(entity: Entity, queryId: QueryId): void;
  pushRemove(entity: Entity, queryId: QueryId): void;
}

/** Vec accumulator -- collects all items. Rust: `impl UpdateItemAccumulator for Vec<ReactorUpdateItem>` */
export class VecAccumulator implements UpdateItemAccumulator {
  readonly items: ReactorUpdateItem[] = [];

  pushInitial(entity: Entity, queryId: QueryId): void {
    this.items.push({
      entity,
      events: [],
      predicateRelevance: [[queryId, 'Initial']],
    });
  }

  pushRemove(entity: Entity, queryId: QueryId): void {
    this.items.push({
      entity,
      events: [],
      predicateRelevance: [[queryId, 'Remove']],
    });
  }
}

/** No-op accumulator -- discards everything. Rust: `impl UpdateItemAccumulator for ()` */
export class NoopAccumulator implements UpdateItemAccumulator {
  pushInitial(_entity: Entity, _queryId: QueryId): void {}
  pushRemove(_entity: Entity, _queryId: QueryId): void {}
}

// ── GapFillData ───────────────────────────────────────────────────────
// Mirrors Rust type alias GapFillData<E>

interface GapFillData {
  queryId: QueryId;
  gapFetcher: GapFetcher;
  collectionId: CollectionId;
  selection: Selection;
  resultset: EntityResultSet;
  lastEntity: Entity | null;
  gapSize: number;
}

// ── QueryState ────────────────────────────────────────────────────────
// Mirrors Rust struct QueryState<E: AbstractEntity + Filterable>

/**
 * Per-query state within a subscription.
 *
 * Rust: `pub struct QueryState<E: AbstractEntity + Filterable>`
 * Divergence: No generic E -- uses concrete Entity [E8].
 */
export interface QueryState {
  /**
   * The query's own ID. Stored here because JS Map keys are strings (queryId.toUlidString()),
   * so we need the actual QueryId object for watcher management and gap filling.
   * Divergence: Rust stores QueryId as the HashMap key and accesses it during iteration;
   *   TS stores it inside the value [E8].
   */
  queryId: QueryId;
  collectionId: CollectionId;
  /** Selection is null until first update_query call (after register_query). */
  selection: Selection | null;
  gapFetcher: GapFetcher;
  /** When true, skip notifications (used during initialization and updates). */
  paused: boolean;
  resultset: EntityResultSet;
  version: number;
}

// ── Subscription ──────────────────────────────────────────────────────
// Mirrors Rust struct Subscription<E, Ev> which wraps Arc<Inner<E, Ev>>.
// Divergence: No Arc -- plain class instance (single-threaded JS) [E8].
// Divergence: No generic E, Ev -- uses concrete Entity and Attested<Event> [E8].
// Divergence: No Mutex on state -- single-threaded JS [E8].

/**
 * State container for a single reactor subscription.
 * Manages queries, entity subscriptions, entity cache, and broadcasts.
 *
 * Rust: `pub(super) struct Subscription<E, Ev>`
 * Divergence: No Arc/Mutex -- single-threaded JS [E8].
 * Divergence: No generic parameters -- concrete types [E8].
 */
export class Subscription {
  readonly _id: ReactorSubscriptionId;
  private _queries: Map<string, QueryState> = new Map();
  private _entitySubscriptions: Set<string> = new Set();
  private _entities: Map<string, Entity> = new Map();
  private _broadcast: Broadcast<ReactorUpdate>;
  private _watcherSet: WatcherSet;

  constructor(
    broadcast: Broadcast<ReactorUpdate>,
    watcherSet: WatcherSet,
  ) {
    this._id = ReactorSubscriptionId.new();
    this._broadcast = broadcast;
    this._watcherSet = watcherSet;
  }

  /** Get the subscription ID. Rust: `pub fn id(&self) -> ReactorSubscriptionId` */
  id(): ReactorSubscriptionId {
    return this._id;
  }

  // ── Entity subscription management ──────────────────────────────────

  /** Add entity subscription. Rust: `pub fn add_entity_subscription(&self, entity_id)` */
  addEntitySubscription(entityId: EntityId): void {
    this._entitySubscriptions.add(entityId.toBase64());
  }

  /** Remove entity subscription. Rust: `pub fn remove_entity_subscription(&self, entity_id)` */
  removeEntitySubscription(entityId: EntityId): void {
    this._entitySubscriptions.delete(entityId.toBase64());
  }

  /** Check if any queries match this entity. Rust: `pub fn any_query_matches(&self, entity_id)` */
  anyQueryMatches(entityId: EntityId): boolean {
    for (const q of this._queries.values()) {
      if (q.resultset.containsKey(entityId)) {
        return true;
      }
    }
    return false;
  }

  // ── System reset ────────────────────────────────────────────────────

  /** System reset -- clear all matching entities and notify. Rust: `pub fn system_reset(&self)` */
  systemReset(): void {
    const updateItems: ReactorUpdateItem[] = [];

    for (const [_queryIdStr, queryState] of this._queries) {
      // For each entity that was matching this query
      for (const entityId of queryState.resultset.keys()) {
        const entityKey = entityId.toBase64();
        const entity = this._entities.get(entityKey);
        if (entity) {
          updateItems.push({
            entity,
            events: [],
            predicateRelevance: [[queryState.queryId, 'Remove']],
          });
        }
      }

      // Clear the matching entities for this query
      queryState.resultset.clear();
      queryState.resultset.setLoaded(false);
    }

    // Clear entity subscriptions and cached entities
    this._entitySubscriptions.clear();
    this._entities.clear();

    // Send the notification if there were any updates
    if (updateItems.length > 0) {
      this._broadcast.send({ items: updateItems });
    }
  }

  /** Get the number of queries for debugging. Rust: `pub fn queries_len(&self)` */
  queriesLen(): number {
    return this._queries.size;
  }

  // ── Query lifecycle ─────────────────────────────────────────────────

  /**
   * Register a new query with empty resultset.
   * Rust: `pub fn register_query(&self, query_id, collection_id, resultset, gap_fetcher)`
   */
  registerQuery(
    queryId: QueryId,
    collectionId: CollectionId,
    resultset: EntityResultSet,
    gapFetcher: GapFetcher,
  ): void {
    const key = queryId.toUlidString();
    if (this._queries.has(key)) {
      throw new Error(`Query ${queryId} already exists`);
    }
    this._queries.set(key, {
      queryId,
      collectionId,
      selection: null,
      gapFetcher,
      paused: false,
      resultset,
      version: 0,
    });
  }

  /**
   * Update predicate watchers for a query (index/wildcard watchers).
   * Rust: `pub fn update_predicate_watchers(&self, query_id, collection_id, old_predicate, new_predicate)`
   */
  updatePredicateWatchers(
    queryId: QueryId,
    collectionId: CollectionId,
    oldPredicate: Predicate | null,
    newPredicate: Predicate,
  ): void {
    const watcherId: WatcherIdPair = { subscriptionId: this._id, queryId };

    if (oldPredicate !== null) {
      this._watcherSet.recursePredicateWatchers(collectionId, oldPredicate, watcherId, 'Remove');
    }
    this._watcherSet.recursePredicateWatchers(collectionId, newPredicate, watcherId, 'Add');
  }

  /**
   * Add entity watchers for entities in a query's resultset.
   * Rust: `pub fn add_entity_watchers(&self, query_id, entity_ids)`
   */
  addEntityWatchers(queryId: QueryId, entityIds: Iterable<EntityId>): void {
    this._watcherSet.addPredicateEntityWatchers(this._id, queryId, entityIds);
  }

  /**
   * Update an existing query.
   * Handles watcher management internally (both predicate and entity watchers).
   * Returns newly_added_entities for server delta generation.
   *
   * Rust: `pub fn update_query<A: UpdateItemAccumulator>(&self, ...)`
   */
  updateQuery(
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    includedEntities: Entity[],
    version: number,
    reactorUpdates: UpdateItemAccumulator,
  ): Entity[] {
    const queryKey = queryId.toUlidString();
    const queryState = this._queries.get(queryKey);
    if (!queryState) {
      throw new Error('Query not found for update');
    }

    // Check if this is the first update (selection is null)
    const isFirstUpdate = queryState.selection === null;

    // Save old selection for comparison
    const oldSelection = queryState.selection;
    queryState.selection = selection;

    // Update resultset configuration
    if (selection.orderBy !== null) {
      const keySpec = buildKeySpecFromSelection(selection.orderBy, queryState.resultset);
      queryState.resultset.orderBy(keySpec);
    } else {
      queryState.resultset.orderBy(null);
    }

    // Set limit if this is first update OR if limit changed
    if (isFirstUpdate || (oldSelection !== null && oldSelection.limit !== selection.limit)) {
      queryState.resultset.setLimit(selection.limit);
    }

    // Create write guard for atomic updates
    const newlyAdded: Entity[] = [];
    const removedEntities: EntityId[] = [];
    {
      using rwResultset = queryState.resultset.write();

      // Mark all entities dirty for re-evaluation
      rwResultset.markAllDirty();

      // Process included entities (only truly new ones from remote)
      for (const entity of includedEntities) {
        if (evaluatePredicate(entityAsFilterable(entity), selection.predicate)) {
          const entityId = entity.id();

          // Check if this is truly new to the resultset
          if (!rwResultset.contains(entityId)) {
            rwResultset.add(entity);
            this._entities.set(entityId.toBase64(), entity);
            this._entitySubscriptions.add(entityId.toBase64());
            reactorUpdates.pushInitial(entity, queryId);
            newlyAdded.push(entity);
          }
        }
      }

      // Remove entities that no longer match the new predicate
      rwResultset.retainDirty((entity: Entity) => {
        if (evaluatePredicate(entityAsFilterable(entity), selection.predicate)) {
          return true;
        }
        const entityId = entity.id();
        removedEntities.push(entityId);
        reactorUpdates.pushRemove(entity, queryId);
        return false;
      });

      // Unpause now that update is complete
      queryState.paused = false;
      queryState.version = version;

      // Set loaded as part of the write transaction
      rwResultset.setLoaded(true);
    } // rwResultset disposed here -> broadcasts if changed

    // Update predicate watchers (setup on first update, or update if predicate changed)
    let shouldUpdateWatchers = false;
    if (isFirstUpdate) {
      shouldUpdateWatchers = true;
    } else if (oldSelection !== null) {
      // Compare predicates -- simple reference check first, then structural
      shouldUpdateWatchers = oldSelection.predicate !== selection.predicate;
    }

    if (shouldUpdateWatchers) {
      const oldPred = oldSelection !== null ? oldSelection.predicate : null;
      this.updatePredicateWatchers(queryId, collectionId, oldPred, selection.predicate);
    }

    // Add entity watchers for newly added entities
    if (newlyAdded.length > 0) {
      this.addEntityWatchers(queryId, newlyAdded.map((e) => e.id()));
    }

    // Remove entity watchers for removed entities
    if (removedEntities.length > 0) {
      this._watcherSet.cleanupRemovedPredicateWatchers(this._id, queryId, removedEntities);
    }

    return newlyAdded;
  }

  /** Send ReactorUpdate with the given items. Rust: `pub fn send_update(&self, items)` */
  sendUpdate(items: ReactorUpdateItem[]): void {
    this._broadcast.send({ items });
  }

  /** Remove a query and return its state for cleanup. Rust: `pub fn remove_query(&self, query_id)` */
  removeQuery(queryId: QueryId): QueryState | undefined {
    const key = queryId.toUlidString();
    const state = this._queries.get(key);
    if (state) {
      this._queries.delete(key);
    }
    return state;
  }

  /** Get all queries for cleanup (used by unsubscribe). Rust: `pub fn take_all_queries(&self)` */
  takeAllQueries(): Map<string, QueryState> {
    const queries = this._queries;
    this._queries = new Map();
    return queries;
  }

  // ── evaluate_changes ────────────────────────────────────────────────

  /**
   * Evaluate candidate changes for this subscription and spawn gap filling/notification.
   * Returns watcher changes that need to be applied to the global WatcherSet.
   *
   * Rust: `pub async fn evaluate_changes<C: ChangeNotification>(...)`
   * Divergence: Not truly async in single-threaded JS for the evaluation phase,
   *   but gap filling returns a Promise [E8].
   */
  async evaluateChanges(
    candidates: CandidateChanges<ChangeNotification>,
  ): Promise<WatcherChange[]> {
    const watcherChanges: WatcherChange[] = [];
    // Use Map<string, ReactorUpdateItem> to preserve insertion order and dedup by entity ID
    const items = new Map<string, ReactorUpdateItem>();

    // Process query-specific candidates using direct lookup
    for (const queryCandidate of candidates.queryIter()) {
      const queryId = queryCandidate.queryId;
      const queryKey = queryId.toUlidString();

      // Direct lookup -- skip if query doesn't exist or is paused
      const queryState = this._queries.get(queryKey);
      if (!queryState || queryState.paused) continue;

      const selection = queryState.selection;
      if (!selection) {
        throw new Error('evaluate_changes called before update_query');
      }

      // Process all candidate changes for this query
      for (const change of queryCandidate.iter()) {
        const entity = change.entity();
        const entityId = entity.id();
        const entityKey = entityId.toBase64();

        const matches = evaluatePredicate(entityAsFilterable(entity), selection.predicate);
        const didMatch = queryState.resultset.containsKey(entityId);

        // Process membership change in one match
        let membershipChange: MembershipChange | null = null;

        if (!didMatch && matches) {
          // Entity now matches -- add to matching set
          { using rw = queryState.resultset.write(); rw.add(entity); }
          this._entities.set(entityKey, entity);
          watcherChanges.push(watcherChangeAdd(entityId, this._id, queryId));
          membershipChange = 'Add';
        } else if (didMatch && !matches) {
          // Entity no longer matches -- remove from matching set
          { using rw = queryState.resultset.write(); rw.remove(entityId); }
          watcherChanges.push(watcherChangeRemove(entityId, this._id, queryId));
          membershipChange = 'Remove';
        } else {
          // No membership change -- just track watcher state
          if (matches) {
            watcherChanges.push(watcherChangeAdd(entityId, this._id, queryId));
          } else {
            watcherChanges.push(watcherChangeRemove(entityId, this._id, queryId));
          }
        }

        // Emit if matches, matched before, or explicitly subscribed
        const entitySubscribed = this._entitySubscriptions.has(entityKey);
        if (matches || didMatch || entitySubscribed) {
          let item = items.get(entityKey);
          if (!item) {
            item = {
              entity,
              events: [...change.events()],
              predicateRelevance: [],
            };
            items.set(entityKey, item);
          }

          if (membershipChange !== null) {
            item.predicateRelevance.push([queryId, membershipChange]);
          }
        }
      }
    }

    // Process entity-level subscriptions not covered by query processing
    for (const change of candidates.entityIter()) {
      const entity = change.entity();
      const entityId = entity.id();
      const entityKey = entityId.toBase64();

      if (this._entitySubscriptions.has(entityKey)) {
        if (!items.has(entityKey)) {
          items.set(entityKey, {
            entity,
            events: [...change.events()],
            predicateRelevance: [],
          });
        }
      }
    }

    // Collect gap fill data
    const gapsToFill = this.collectGapsToFillInternal();

    // Collect update items
    const updateItems: ReactorUpdateItem[] = Array.from(items.values());

    if (gapsToFill.length > 0) {
      // Spawn gap filling and notification as an async task
      // Divergence: No crate::task::spawn -- just kick off the async work [E8].
      this.fillGapsAndNotify(updateItems, gapsToFill);
    } else if (updateItems.length > 0) {
      this._broadcast.send({ items: updateItems });
    }

    return watcherChanges;
  }

  // ── Gap filling ─────────────────────────────────────────────────────

  /**
   * Collect gaps to fill (internal version).
   * Rust: `fn collect_gaps_to_fill_internal(&self, state)`
   */
  private collectGapsToFillInternal(): GapFillData[] {
    const result: GapFillData[] = [];
    for (const [_queryKey, queryState] of this._queries) {
      const gapData = this.extractGapData(queryState);
      if (gapData !== null) {
        result.push(gapData);
      }
    }
    return result;
  }

  /**
   * Extract gap data for a single query.
   * Rust: `fn extract_gap_data(&self, query_id, query_state)`
   * Divergence: queryId is stored inside QueryState rather than being the map key [E8].
   */
  private extractGapData(queryState: QueryState): GapFillData | null {
    const resultset = queryState.resultset;

    if (!resultset.isGapDirty()) {
      return null;
    }

    const limit = resultset.getLimit();
    if (limit === null) {
      return null;
    }

    const currentLen = resultset.len();
    if (currentLen >= limit) {
      return null;
    }

    const gapSize = limit - currentLen;
    const lastEntity = resultset.lastEntity();

    const selection = queryState.selection;
    if (selection === null) {
      throw new Error('extract_gap_data called before update_query');
    }

    return {
      queryId: queryState.queryId,
      gapFetcher: queryState.gapFetcher,
      collectionId: queryState.collectionId,
      selection,
      resultset,
      lastEntity,
      gapSize,
    };
  }

  /**
   * Fill gaps for a specific query and append entities to the provided array.
   * Also registers entity watchers for gap-filled entities.
   *
   * Rust: `pub async fn fill_gaps_for_query_entities(&self, query_id, entities)`
   */
  async fillGapsForQueryEntities(queryId: QueryId, entities: Entity[]): Promise<void> {
    const queryKey = queryId.toUlidString();
    const queryState = this._queries.get(queryKey);
    if (!queryState) return;

    const gapData = this.extractGapData(queryState);
    if (gapData === null) return;

    // Clear gap_dirty flag immediately
    gapData.resultset.clearGapDirty();

    // Process gap fill
    const gapFilledEntities = await Subscription.processGapFillEntities(gapData);

    // Register entity watchers and append entities
    if (gapFilledEntities.length > 0) {
      this.addEntityWatchers(queryId, gapFilledEntities.map((e) => e.id()));
      entities.push(...gapFilledEntities);
    }
  }

  /**
   * Fill gaps for a specific query and push ReactorUpdateItems to the accumulator.
   * Also registers entity watchers for gap-filled entities.
   *
   * Rust: `pub async fn fill_gaps_for_query<A: UpdateItemAccumulator>(&self, query_id, reactor_updates)`
   */
  async fillGapsForQuery(queryId: QueryId, reactorUpdates: UpdateItemAccumulator): Promise<void> {
    const queryKey = queryId.toUlidString();
    const queryState = this._queries.get(queryKey);
    if (!queryState) return;

    const gapData = this.extractGapData(queryState);
    if (gapData === null) return;

    // Clear gap_dirty flag immediately
    gapData.resultset.clearGapDirty();

    // Process gap fill
    const gapFilledEntities = await Subscription.processGapFillEntities(gapData);

    // Register entity watchers and push items for gap-filled entities
    if (gapFilledEntities.length > 0) {
      this.addEntityWatchers(queryId, gapFilledEntities.map((e) => e.id()));

      for (const entity of gapFilledEntities) {
        reactorUpdates.pushInitial(entity, queryId);
      }
    }
  }

  /**
   * Process gap fill entities (static async helper).
   * Rust: `async fn process_gap_fill_entities(...)`
   */
  private static async processGapFillEntities(gap: GapFillData): Promise<Entity[]> {
    try {
      const gapEntities = await gap.gapFetcher.fetchGap(
        gap.collectionId,
        gap.selection,
        gap.lastEntity,
        gap.gapSize,
      );

      if (gapEntities.length > 0) {
        const addedEntities: Entity[] = [];
        {
          using rw = gap.resultset.write();
          for (const entity of gapEntities) {
            if (rw.add(entity)) {
              addedEntities.push(entity);
            }
          }
        }
        return addedEntities;
      }

      return [];
    } catch (e) {
      console.warn(`Gap filling failed for query ${gap.queryId}: ${e}`);
      return [];
    }
  }

  /**
   * Fill gaps and send notification.
   * Combined method to handle gap filling and notification in a single task.
   *
   * Rust: `async fn fill_gaps_and_notify(self, items, gaps_to_fill, broadcast)`
   * Divergence: fire-and-forget async (no task::spawn in JS) [E8].
   */
  private async fillGapsAndNotify(
    items: ReactorUpdateItem[],
    gapsToFill: GapFillData[],
  ): Promise<void> {
    // Clear gap_dirty flags immediately for all queries
    for (const gap of gapsToFill) {
      gap.resultset.clearGapDirty();
    }

    // Process all gap fills concurrently
    const gapFillPromises = gapsToFill.map((gap) =>
      Subscription.processGapFill(gap),
    );

    const gapResults = await Promise.all(gapFillPromises);

    // Collect all the new items from gap filling and register entity watchers
    for (const { queryId, gapItems } of gapResults) {
      if (gapItems.length > 0) {
        // Register entity watchers for gap-filled entities
        const entityIds = gapItems.map((item) => item.entity.id());
        this.addEntityWatchers(queryId, entityIds);

        items.push(...gapItems);
      }
    }

    // Send the consolidated update
    if (items.length > 0) {
      this._broadcast.send({ items });
    }
  }

  /**
   * Process a single gap fill (static async helper).
   * Rust: `async fn process_gap_fill(...)`
   */
  private static async processGapFill(
    gap: GapFillData,
  ): Promise<{ queryId: QueryId; gapItems: ReactorUpdateItem[] }> {
    try {
      const gapEntities = await gap.gapFetcher.fetchGap(
        gap.collectionId,
        gap.selection,
        gap.lastEntity,
        gap.gapSize,
      );

      if (gapEntities.length > 0) {
        const gapItems: ReactorUpdateItem[] = [];
        {
          using rw = gap.resultset.write();
          for (const entity of gapEntities) {
            if (rw.add(entity)) {
              gapItems.push({
                entity,
                events: [],
                predicateRelevance: [[gap.queryId, 'Add']],
              });
            }
          }
        }
        return { queryId: gap.queryId, gapItems };
      }

      return { queryId: gap.queryId, gapItems: [] };
    } catch (e) {
      console.warn(`Gap filling failed for query ${gap.queryId}: ${e}`);
      return { queryId: gap.queryId, gapItems: [] };
    }
  }
}

// ── buildKeySpecFromSelection ─────────────────────────────────────────
// Mirrors Rust pub(crate) fn build_key_spec_from_selection<E: AbstractEntity>(...)

/**
 * Build KeySpec from Selection's ORDER BY clause with type inference from sample entities.
 *
 * Rust: `pub(crate) fn build_key_spec_from_selection(order_by, resultset) -> Result<KeySpec>`
 * Divergence: Returns KeySpec directly (throws on error) [E7].
 */
export function buildKeySpecFromSelection(
  orderBy: OrderByItem[],
  resultset: EntityResultSet,
): KeySpec {
  const keyparts: IndexKeyPart[] = [];

  const read = resultset.read();
  for (const item of orderBy) {
    // Use the property name from the path
    const column = item.path.property();

    // Infer type from first non-null value in resultset entities
    let inferredType = ValueType.String; // default
    for (const [, entity] of read.iterEntities()) {
      const val = entity.getPropertyValue(column);
      if (val !== null) {
        inferredType = valueType(val);
        break;
      }
    }

    const direction: IndexDirection =
      item.direction === 'Asc' ? IndexDirection.Asc : IndexDirection.Desc;

    keyparts.push({
      column,
      subPath: null,
      direction,
      valueType: inferredType,
      nulls: NullsOrder.Last,
      collation: null,
    });
  }

  return { keyparts };
}
