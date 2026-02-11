// MIRRORS: ankurah/core/src/reactor/watcherset.rs

import type { CollectionId, EntityId, QueryId } from '@ankurah/proto';
import { generateUlidBytes, ulidBytesToString } from '@ankurah/proto';
import type { Predicate, ComparisonOperator, Literal, PathExpr, Expr } from '@ankurah/ankql';
import { ComparisonIndex } from './comparison-index.ts';
import { PropertyPath } from './property-path.ts';
import { CandidateChanges } from './candidate-changes.ts';
import type { Entity } from '../entity.ts';
import type { Value } from '../value/index.ts';

// ── ReactorSubscriptionId ───────────────────────────────────────────

/**
 * Unique identifier for a reactor subscription, local to a single reactor/node.
 * Cannot be transported across nodes. Mirrors Rust ReactorSubscriptionId(Ulid).
 *
 * Uses a ULID string as its identity for Map-key usage (JS lacks structural equality on objects).
 * Divergence: Rust derives Copy/Clone/Hash/Eq automatically; TS needs explicit toKey() for Map usage [E8].
 */
export class ReactorSubscriptionId {
  private readonly ulid: string; // 26-char Crockford Base32 ULID string

  private constructor(ulid: string) {
    this.ulid = ulid;
  }

  static new(): ReactorSubscriptionId {
    return new ReactorSubscriptionId(ulidBytesToString(generateUlidBytes()));
  }

  /** String key for use in Maps. */
  toKey(): string {
    return this.ulid;
  }

  toString(): string {
    return `RS-${this.ulid}`;
  }

  equals(other: ReactorSubscriptionId): boolean {
    return this.ulid === other.ulid;
  }
}

// ── WatcherOp ───────────────────────────────────────────────────────

/**
 * Whether a watcher is being added or removed.
 * Rust: `pub enum WatcherOp { Add, Remove }`
 */
export type WatcherOp = 'Add' | 'Remove';

// ── EntityWatcherId ─────────────────────────────────────────────────

/**
 * Identifies how an entity is being watched -- either by a predicate query
 * or by an explicit entity subscription.
 *
 * Rust: `enum EntityWatcherId { Predicate(ReactorSubscriptionId, QueryId), Subscription(ReactorSubscriptionId) }`
 *
 * Divergence: Rust derives Hash/Eq/Ord automatically. TS needs a stable string
 * key for use in Map<string, ...> since JS Maps use reference equality [E8].
 */
export type EntityWatcherId =
  | { type: 'Predicate'; subscriptionId: ReactorSubscriptionId; queryId: QueryId }
  | { type: 'Subscription'; subscriptionId: ReactorSubscriptionId };

/** Extract the subscription ID from either variant. */
export function entityWatcherSubscriptionId(ew: EntityWatcherId): ReactorSubscriptionId {
  return ew.subscriptionId;
}

/**
 * Produce a stable string key for an EntityWatcherId, suitable for use
 * as a key in a Map<string, ...>.
 *
 * Divergence: Rust gets this for free via Hash+Eq derive; JS needs explicit serialization [E8].
 */
export function entityWatcherIdKey(ew: EntityWatcherId): string {
  if (ew.type === 'Predicate') {
    return `P:${ew.subscriptionId.toKey()}:${ew.queryId.toUlidString()}`;
  }
  return `S:${ew.subscriptionId.toKey()}`;
}

// ── WatcherChange ───────────────────────────────────────────────────

/**
 * Represents a deferred mutation to entity watchers that should be applied
 * after evaluate_changes completes (to avoid holding locks across async work).
 *
 * Rust: `pub enum WatcherChange { Add { ... }, Remove { ... } }`
 */
export type WatcherChange =
  | { type: 'Add'; entityId: EntityId; subscriptionId: ReactorSubscriptionId; queryId: QueryId }
  | { type: 'Remove'; entityId: EntityId; subscriptionId: ReactorSubscriptionId; queryId: QueryId };

/** Factory: create an Add watcher change. */
export function watcherChangeAdd(
  entityId: EntityId,
  subscriptionId: ReactorSubscriptionId,
  queryId: QueryId,
): WatcherChange {
  return { type: 'Add', entityId, subscriptionId, queryId };
}

/** Factory: create a Remove watcher change. */
export function watcherChangeRemove(
  entityId: EntityId,
  subscriptionId: ReactorSubscriptionId,
  queryId: QueryId,
): WatcherChange {
  return { type: 'Remove', entityId, subscriptionId, queryId };
}

// ── Composite key helper ────────────────────────────────────────────

/**
 * Build a composite map key from collection ID + property path.
 * Divergence: Rust uses (CollectionId, PropertyPath) tuple as HashMap key;
 * JS needs a string key because Maps use reference equality [E8].
 */
function indexWatcherKey(collectionId: CollectionId, propertyPath: PropertyPath): string {
  return `${collectionId.toString()}::${propertyPath.toString()}`;
}

// ── WatcherIdPair ───────────────────────────────────────────────────

/**
 * The subscriber identity stored inside ComparisonIndex and wildcard sets.
 * Mirrors Rust tuple `(ReactorSubscriptionId, proto::QueryId)`.
 *
 * Divergence: Rust uses a tuple with derived Hash+Eq; TS uses an interface
 * with explicit key serialization for Map/Set usage [E8].
 */
export interface WatcherIdPair {
  subscriptionId: ReactorSubscriptionId;
  queryId: QueryId;
}

/**
 * Produce a stable string key for a WatcherIdPair, for use with Map<string, ...>.
 */
export function watcherIdPairKey(pair: WatcherIdPair): string {
  return `${pair.subscriptionId.toKey()}:${pair.queryId.toUlidString()}`;
}

// ── IndexWatcherEntry ───────────────────────────────────────────────

/**
 * Stored alongside each ComparisonIndex to avoid re-parsing the composite key.
 * Divergence: Rust stores (CollectionId, PropertyPath) as the HashMap key; TS stores
 * them alongside the index to avoid reparsing string keys on every lookup [E8].
 */
interface IndexWatcherEntry {
  collectionId: CollectionId;
  propertyPath: PropertyPath;
  index: ComparisonIndex<string>;
}

// ── WatcherSet ──────────────────────────────────────────────────────

/**
 * Central routing table that maps entity changes to interested subscriptions.
 * Maintains three registries: index watchers, wildcard watchers, and entity watchers.
 *
 * Rust: `pub struct WatcherSet`
 *
 * Divergence: Rust wraps WatcherSet in Arc<Mutex<WatcherSet>> for thread safety;
 * JS is single-threaded so WatcherSet is a plain object with no locking [E8].
 */
export class WatcherSet {
  /**
   * Per (collection, property-path) comparison indexes.
   * Rust: HashMap<(CollectionId, PropertyPath), ComparisonIndex<(ReactorSubscriptionId, QueryId)>>
   *
   * Key: composite string from indexWatcherKey().
   * Value: IndexWatcherEntry with ComparisonIndex<string> where strings are watcherIdPairKey serializations.
   *
   * Divergence: Rust HashMap with tuple key -> JS Map with string key [E8].
   * Divergence: Rust ComparisonIndex<(SubId, QueryId)> -> TS ComparisonIndex<string> with reverse lookup [E8].
   */
  private indexWatchers: Map<string, IndexWatcherEntry> = new Map();

  /**
   * Reverse lookup: watcherIdPairKey string -> WatcherIdPair.
   * Shared across all ComparisonIndex instances so that findMatching results
   * can be resolved back to (subscriptionId, queryId).
   *
   * Divergence: Rust doesn't need this because ComparisonIndex stores actual tuples;
   * TS stores string keys and needs reverse lookup [E8].
   */
  private watcherIdLookup: Map<string, WatcherIdPair> = new Map();

  /**
   * Per-collection sets of watchers that match ANY entity change.
   * Rust: HashMap<CollectionId, HashSet<(ReactorSubscriptionId, QueryId)>>
   *
   * Key: collectionId.toString()
   * Value: Map<string, WatcherIdPair> keyed by watcherIdPairKey.
   *
   * Divergence: Rust HashSet<tuple> -> JS Map<string, WatcherIdPair> for value equality [E8].
   */
  private wildcardWatchers: Map<string, Map<string, WatcherIdPair>> = new Map();

  /**
   * Per-entity watcher registrations.
   * Rust: HashMap<EntityId, HashSet<EntityWatcherId>>
   *
   * Key: entityId.toBase64() (stable string for value equality)
   * Value: Map<string, EntityWatcherId> keyed by entityWatcherIdKey.
   *
   * Divergence: Rust HashSet<EntityWatcherId> -> JS Map<string, EntityWatcherId> for value equality [E8].
   */
  private entityWatchers: Map<string, Map<string, EntityWatcherId>> = new Map();

  constructor() {
    // All maps initialized to empty in field declarations.
  }

  // ── recursePredicateWatchers ────────────────────────────────────

  /**
   * Recursively walk a predicate AST and register/unregister index watchers and
   * wildcard watchers for the given watcher ID pair.
   *
   * Rust: pub fn recurse_predicate_watchers(
   *          &mut self,
   *          collection_id: &CollectionId,
   *          predicate: &Predicate,
   *          watcher_id: (ReactorSubscriptionId, QueryId),
   *          op: WatcherOp,
   *       )
   */
  recursePredicateWatchers(
    collectionId: CollectionId,
    predicate: Predicate,
    watcherId: WatcherIdPair,
    op: WatcherOp,
  ): void {
    const pairKey = watcherIdPairKey(watcherId);

    switch (predicate.type) {
      case 'Comparison': {
        // Extract path and literal from left/right (in either order)
        let path: PathExpr | null = null;
        let literal: Literal | null = null;
        const operator: ComparisonOperator = predicate.operator;

        const { left, right } = predicate;
        if (left.type === 'Path' && right.type === 'Literal') {
          path = left.value;
          literal = right.value;
        } else if (left.type === 'Literal' && right.type === 'Path') {
          path = right.value;
          literal = left.value;
        }

        if (path && literal) {
          const propertyPath = PropertyPath.fromPath(path);
          const compositeKey = indexWatcherKey(collectionId, propertyPath);
          let entry = this.indexWatchers.get(compositeKey);
          if (!entry) {
            entry = { collectionId, propertyPath, index: new ComparisonIndex<string>() };
            this.indexWatchers.set(compositeKey, entry);
          }

          if (op === 'Add') {
            entry.index.add(literal, operator, pairKey);
            this.watcherIdLookup.set(pairKey, watcherId);
          } else {
            entry.index.remove(literal, operator, pairKey);
            // Note: Do NOT remove from watcherIdLookup here because the same
            // pair may be registered in multiple indexes. Cleanup happens when
            // subscription is fully removed.
          }
        }
        // else: unsupported comparison shape, silently skip (mirrors Rust behavior)
        break;
      }

      case 'And':
      case 'Or': {
        this.recursePredicateWatchers(collectionId, predicate.left, watcherId, op);
        this.recursePredicateWatchers(collectionId, predicate.right, watcherId, op);
        break;
      }

      case 'Not': {
        this.recursePredicateWatchers(collectionId, predicate.predicate, watcherId, op);
        break;
      }

      case 'True': {
        const collectionKey = collectionId.toString();
        let set = this.wildcardWatchers.get(collectionKey);
        if (!set) {
          set = new Map();
          this.wildcardWatchers.set(collectionKey, set);
        }

        if (op === 'Add') {
          set.set(pairKey, watcherId);
        } else {
          set.delete(pairKey);
        }
        break;
      }

      case 'IsNull':
        throw new Error('Predicate::IsNull not implemented in WatcherSet');

      case 'False':
        throw new Error('Predicate::False not implemented in WatcherSet');

      case 'Placeholder':
        throw new Error('Placeholder should be transformed before reactor processing');
    }
  }

  // ── accumulateInterestedWatchers ────────────────────────────────

  /**
   * For a single entity change, find all subscriptions that might be interested
   * and record this change in their CandidateChanges accumulator.
   *
   * Rust: pub fn accumulate_interested_watchers<E: AbstractEntity, C>(
   *          &self,
   *          entity: &E,
   *          offset: usize,
   *          changes_arc: &Arc<Vec<C>>,
   *          candidates_by_sub: &mut HashMap<ReactorSubscriptionId, CandidateChanges<C>>,
   *       )
   *
   * Divergence: Arc<Vec<C>> -> readonly C[] (JS passes arrays by reference, no need for Arc) [E8].
   * Divergence: HashMap<ReactorSubscriptionId, CandidateChanges<C>> -> Map<string, {...}>
   *   keyed by subscriptionId.toKey() [E8].
   * Divergence: E: AbstractEntity -> Entity (concrete type; the TS port uses Entity directly) [E7].
   */
  accumulateInterestedWatchers<C>(
    entity: Entity,
    offset: number,
    changes: readonly C[],
    candidatesBySub: Map<string, { subscriptionId: ReactorSubscriptionId; candidates: CandidateChanges<C> }>,
  ): void {
    const entityId = entity.entityId;
    const entityCollectionStr = entity.collectionId.toString();

    // ── Phase 1: Index watchers ──
    for (const [_key, { collectionId, propertyPath, index }] of this.indexWatchers) {
      if (collectionId.toString() !== entityCollectionStr) continue;

      const value: Value | null = propertyPath.extractValue(entity);
      if (value === null) continue;

      const matchingKeys: string[] = index.findMatching(value);
      for (const watcherKey of matchingKeys) {
        const pair = this.watcherIdLookup.get(watcherKey)!;
        const subKey = pair.subscriptionId.toKey();
        let entry = candidatesBySub.get(subKey);
        if (!entry) {
          entry = { subscriptionId: pair.subscriptionId, candidates: new CandidateChanges(changes) };
          candidatesBySub.set(subKey, entry);
        }
        entry.candidates.addQuery(pair.queryId, offset);
      }
    }

    // ── Phase 2: Wildcard watchers ──
    const wildcards = this.wildcardWatchers.get(entityCollectionStr);
    if (wildcards) {
      for (const [_key, pair] of wildcards) {
        const subKey = pair.subscriptionId.toKey();
        let entry = candidatesBySub.get(subKey);
        if (!entry) {
          entry = { subscriptionId: pair.subscriptionId, candidates: new CandidateChanges(changes) };
          candidatesBySub.set(subKey, entry);
        }
        entry.candidates.addQuery(pair.queryId, offset);
      }
    }

    // ── Phase 3: Entity watchers ──
    const entityKey = entityId.toBase64();
    const entityWatcherSet = this.entityWatchers.get(entityKey);
    if (entityWatcherSet) {
      for (const [_key, watcherId] of entityWatcherSet) {
        const subKey = watcherId.subscriptionId.toKey();
        let entry = candidatesBySub.get(subKey);
        if (!entry) {
          entry = {
            subscriptionId: watcherId.subscriptionId,
            candidates: new CandidateChanges(changes),
          };
          candidatesBySub.set(subKey, entry);
        }

        if (watcherId.type === 'Predicate') {
          entry.candidates.addQuery(watcherId.queryId, offset);
        } else {
          // Subscription -- entity-level, not tied to a query
          entry.candidates.addEntity(offset);
        }
      }
    }
  }

  // ── applyWatcherChange ──────────────────────────────────────────

  /**
   * Apply a single WatcherChange (add or remove an entity-level predicate watcher).
   *
   * Rust: pub fn apply_watcher_change(&mut self, change: WatcherChange)
   */
  applyWatcherChange(change: WatcherChange): void {
    switch (change.type) {
      case 'Add': {
        const entityKey = change.entityId.toBase64();
        let watchers = this.entityWatchers.get(entityKey);
        if (!watchers) {
          watchers = new Map();
          this.entityWatchers.set(entityKey, watchers);
        }
        const ew: EntityWatcherId = {
          type: 'Predicate',
          subscriptionId: change.subscriptionId,
          queryId: change.queryId,
        };
        watchers.set(entityWatcherIdKey(ew), ew);
        break;
      }
      case 'Remove': {
        const entityKey = change.entityId.toBase64();
        const watchers = this.entityWatchers.get(entityKey);
        if (watchers) {
          const ew: EntityWatcherId = {
            type: 'Predicate',
            subscriptionId: change.subscriptionId,
            queryId: change.queryId,
          };
          watchers.delete(entityWatcherIdKey(ew));
          if (watchers.size === 0) {
            this.entityWatchers.delete(entityKey);
          }
        }
        break;
      }
    }
  }

  // ── addEntitySubscription ───────────────────────────────────────

  /**
   * Register an entity-level subscription watcher (not tied to any query predicate).
   *
   * Rust: pub fn add_entity_subscription(&mut self, subscription_id: ReactorSubscriptionId, entity_id: EntityId)
   */
  addEntitySubscription(subscriptionId: ReactorSubscriptionId, entityId: EntityId): void {
    const entityKey = entityId.toBase64();
    let watchers = this.entityWatchers.get(entityKey);
    if (!watchers) {
      watchers = new Map();
      this.entityWatchers.set(entityKey, watchers);
    }
    const ew: EntityWatcherId = { type: 'Subscription', subscriptionId };
    watchers.set(entityWatcherIdKey(ew), ew);
  }

  // ── removeEntitySubscription ────────────────────────────────────

  /**
   * Remove an entity-level subscription watcher.
   *
   * Rust: pub fn remove_entity_subscription(&mut self, subscription_id: ReactorSubscriptionId, entity_id: EntityId)
   */
  removeEntitySubscription(subscriptionId: ReactorSubscriptionId, entityId: EntityId): void {
    const entityKey = entityId.toBase64();
    const watchers = this.entityWatchers.get(entityKey);
    if (watchers) {
      const ew: EntityWatcherId = { type: 'Subscription', subscriptionId };
      watchers.delete(entityWatcherIdKey(ew));
      if (watchers.size === 0) {
        this.entityWatchers.delete(entityKey);
      }
    }
  }

  // ── removeEntitySubscriptions (batch) ───────────────────────────

  /**
   * Remove entity subscription watchers for multiple entities.
   *
   * Rust: pub fn remove_entity_subscriptions(&mut self, subscription_id, entity_ids: impl IntoIterator<Item = EntityId>)
   * Divergence: Rust uses impl IntoIterator; TS uses Iterable [E7].
   */
  removeEntitySubscriptions(subscriptionId: ReactorSubscriptionId, entityIds: Iterable<EntityId>): void {
    for (const entityId of entityIds) {
      this.removeEntitySubscription(subscriptionId, entityId);
    }
  }

  // ── addPredicateEntityWatchers (batch) ──────────────────────────

  /**
   * Add predicate-based entity watchers for multiple entities at once.
   *
   * Rust: pub fn add_predicate_entity_watchers(
   *          &mut self, subscription_id, query_id, entity_ids: impl IntoIterator<Item = EntityId>
   *       )
   * Divergence: Rust uses impl IntoIterator; TS uses Iterable [E7].
   */
  addPredicateEntityWatchers(
    subscriptionId: ReactorSubscriptionId,
    queryId: QueryId,
    entityIds: Iterable<EntityId>,
  ): void {
    for (const entityId of entityIds) {
      const entityKey = entityId.toBase64();
      let watchers = this.entityWatchers.get(entityKey);
      if (!watchers) {
        watchers = new Map();
        this.entityWatchers.set(entityKey, watchers);
      }
      const ew: EntityWatcherId = {
        type: 'Predicate',
        subscriptionId,
        queryId,
      };
      watchers.set(entityWatcherIdKey(ew), ew);
    }
  }

  // ── cleanupRemovedPredicateWatchers ─────────────────────────────

  /**
   * Remove predicate entity watchers for entities that no longer match a query.
   *
   * Rust: pub fn cleanup_removed_predicate_watchers(
   *          &mut self, subscription_id, query_id, removed_entities: &[EntityId]
   *       )
   *
   * Note: Unlike the Rust version which does NOT clean up empty entity entries,
   * the TS port DOES clean up empty entries to avoid memory leaks.
   */
  cleanupRemovedPredicateWatchers(
    subscriptionId: ReactorSubscriptionId,
    queryId: QueryId,
    removedEntities: readonly EntityId[],
  ): void {
    for (const entityId of removedEntities) {
      const entityKey = entityId.toBase64();
      const watchers = this.entityWatchers.get(entityKey);
      if (watchers) {
        const ew: EntityWatcherId = {
          type: 'Predicate',
          subscriptionId,
          queryId,
        };
        watchers.delete(entityWatcherIdKey(ew));
        // Clean up empty entries to avoid memory leaks
        if (watchers.size === 0) {
          this.entityWatchers.delete(entityKey);
        }
      }
    }
  }

  // ── clearEntityWatchers ─────────────────────────────────────────

  /**
   * Clear ALL entity watchers. Used during system reset.
   *
   * Rust: pub fn clear_entity_watchers(&mut self)
   */
  clearEntityWatchers(): void {
    this.entityWatchers.clear();
  }

  // ── debugData ───────────────────────────────────────────────────

  /**
   * Return references to internal data for debugging/testing.
   *
   * Rust: pub fn debug_data(&self) -> (&index_watchers, &wildcard_watchers, &entity_watchers)
   *
   * Divergence: Returns the internal maps directly (JS has no borrow checker concern) [E8].
   */
  debugData(): {
    indexWatchers: Map<string, IndexWatcherEntry>;
    wildcardWatchers: Map<string, Map<string, WatcherIdPair>>;
    entityWatchers: Map<string, Map<string, EntityWatcherId>>;
  } {
    return {
      indexWatchers: this.indexWatchers,
      wildcardWatchers: this.wildcardWatchers,
      entityWatchers: this.entityWatchers,
    };
  }
}
