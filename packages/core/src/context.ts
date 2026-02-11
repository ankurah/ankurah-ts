// MIRRORS: ankurah/core/src/context.rs

import type { CollectionId, EntityId } from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';
import type { Entity } from './entity.ts';
import type { AccessDenied, MutationError, RetrievalError } from './error.ts';
import type { Transaction } from './transaction.ts';
import type { MatchArgs } from './node.ts';
import type { EntityLiveQuery } from './livequery.ts';

// ---------------------------------------------------------------------------
// TContext — abstract interface for transaction context
// ---------------------------------------------------------------------------

/**
 * Abstract interface for transaction implementation (entity management, commit, storage access).
 *
 * Rust: `pub trait TContext { ... }`
 *
 * This is implemented by NodeAndContext in Layer 4 (Node). For Layer 3,
 * Transaction only depends on this interface, not on Node directly.
 *
 * Divergence: Rust uses async_trait for async methods; TS uses Promise returns [A6].
 * Divergence: Rust uses Arc<AtomicBool> for trxAlive; TS uses { value: boolean } [E8].
 */
export interface TContext {
  /**
   * Get the node's entity ID.
   *
   * Rust: `fn node_id(&self) -> EntityId`
   */
  nodeId(): EntityId;

  /**
   * Create a brand new entity for a transaction, and add it to the WeakEntitySet.
   * Note that this does not actually persist the entity to the storage engine.
   * It merely ensures that there are no duplicate entities with the same ID.
   *
   * Rust: `fn create_entity(&self, collection: CollectionId, trx_alive: Arc<AtomicBool>) -> Entity`
   */
  createEntity(collection: CollectionId, trxAlive: { value: boolean }): Entity;

  /**
   * Check write permissions for an entity.
   *
   * Rust: `fn check_write(&self, entity: &Entity) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  checkWrite(entity: Entity): void;

  /**
   * Retrieve a single entity by ID.
   *
   * Rust: `async fn get_entity(&self, id: EntityId, collection: &CollectionId, cached: bool) -> Result<Entity, RetrievalError>`
   */
  getEntity(id: EntityId, collection: CollectionId, cached: boolean): Promise<Entity>;

  /**
   * Get an entity from the local weak entity set (no storage/network access).
   *
   * Rust: `fn get_resident_entity(&self, id: EntityId) -> Option<Entity>`
   */
  getResidentEntity(id: EntityId): Entity | null;

  /**
   * Fetch multiple entities matching a query.
   *
   * Rust: `async fn fetch_entities(&self, collection: &CollectionId, args: MatchArgs) -> Result<Vec<Entity>, RetrievalError>`
   */
  fetchEntities(collection: CollectionId, args: unknown): Promise<Entity[]>;

  /**
   * Commit a local transaction — the full commit pipeline.
   *
   * Rust: `async fn commit_local_trx(&self, trx: &Transaction) -> Result<(), MutationError>`
   */
  commitLocalTrx(trx: Transaction): Promise<void>;

  /**
   * Create a live query for a collection with the given match args.
   *
   * Rust: `fn query(&self, collection_id: CollectionId, args: MatchArgs) -> Result<EntityLiveQuery, RetrievalError>`
   */
  query(collectionId: CollectionId, args: MatchArgs): EntityLiveQuery;
}

// ---------------------------------------------------------------------------
// Context — public API wrapper
// ---------------------------------------------------------------------------

/**
 * Public API for transaction context.
 *
 * Rust: `pub struct Context(Arc<dyn TContext + Send + Sync + 'static>)`
 *
 * Wraps a TContext and provides the main user-facing API for:
 * - Beginning transactions
 * - Reading entities (outside transactions)
 * - Querying / subscribing
 *
 * Note: Most Context methods that depend on Node/Storage/LiveQuery are deferred
 * to Layer 4. This Layer 3 implementation provides the core transaction-creation
 * and the interface definition.
 */
export class Context {
  private readonly inner: TContext;

  constructor(inner: TContext) {
    this.inner = inner;
  }

  /**
   * Get the node's entity ID.
   *
   * Rust: `pub fn node_id(&self) -> EntityId`
   */
  nodeId(): EntityId {
    return this.inner.nodeId();
  }

  /**
   * Begin a transaction.
   *
   * Rust: `pub fn begin(&self) -> Transaction`
   */
  begin(): Transaction {
    // Inline import to avoid circular dependency (Transaction imports TContext from this file)
    const { Transaction: TransactionClass } = require('./transaction.ts');
    return new TransactionClass(this.inner);
  }

  /**
   * Query entities matching a predicate, returning a LiveQuery.
   *
   * Rust: `pub fn query<R: View>(&self, args: impl TryInto<MatchArgs>) -> Result<LiveQuery<R>, RetrievalError>`
   * [context.rs lines 145-149]
   *
   * Divergence: Takes collectionId + args explicitly instead of using View::Model::collection() [E8].
   */
  query(collectionId: CollectionId, args: MatchArgs): EntityLiveQuery {
    // Rust: self.0.query(R::Model::collection(), args)?.map::<R>()
    // Delegates to NodeAndContext.query() which creates EntityLiveQuery
    return this.inner.query(collectionId, args);
  }

  /**
   * Get the underlying TContext (for internal use).
   */
  get context(): TContext {
    return this.inner;
  }
}
