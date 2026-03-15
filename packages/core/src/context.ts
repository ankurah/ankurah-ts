// MIRRORS: ankurah/core/src/context.rs
import type { CollectionId, EntityId } from '@ankurah/proto';
import type { Entity } from './entity.ts';
import type { EntityLiveQuery, LiveQuery } from './livequery.ts';
import type { ModelDefinition, ViewInstance } from './model.ts';
import type { MatchArgs } from './node.ts';
import type { StorageCollection } from './storage.ts';
import type { Transaction } from './transaction.ts';

// ── TContext ─────────────────────────────────────────────────────────────────
// Rust: #[async_trait] pub trait TContext
// Divergence: No Send/Sync bounds — single-threaded JS [E8].
// Divergence: Rust uses Arc<AtomicBool> for trxAlive; TS uses { value: boolean } [E8].

export interface TContext {
  nodeId(): EntityId;
  // Create a brand new entity for a transaction, and add it to the WeakEntitySet.
  // Note that this does not actually persist the entity to the storage engine.
  // It merely ensures that there are no duplicate entities with the same ID (except forked entities).
  createEntity(collection: CollectionId, trxAlive: { value: boolean }): Entity;
  checkWrite(entity: Entity): void; // Divergence: Rust returns Result<(), AccessDenied>; TS throws [E3]
  getEntity(id: EntityId, collection: CollectionId, cached: boolean): Promise<Entity>;
  getResidentEntity(id: EntityId): Entity | null; // Divergence: Option<T> → T | null [E3]
  fetchEntities(collection: CollectionId, args: MatchArgs): Promise<Entity[]>;
  commitLocalTrx(trx: Transaction): Promise<void>;
  query(collectionId: CollectionId, args: MatchArgs): EntityLiveQuery;
  collection(id: CollectionId): Promise<StorageCollection>;
}

// ── Context ──────────────────────────────────────────────────────────────────
// Rust: pub struct Context(Arc<dyn TContext + Send + Sync + 'static>)

export class Context {
  private readonly inner: TContext;

  constructor(inner: TContext) {
    this.inner = inner;
  }

  // Rust: impl Clone for Context — JS objects are reference types [E8]

  nodeId(): EntityId {
    return this.inner.nodeId();
  }

  // TODO: Fix this - arghhh async lifetimes
  // pub async fn trx<T, F, Fut>(self: &Arc<Self>, f: F) -> anyhow::Result<T>

  // Rust: pub fn begin(&self) -> Transaction
  begin(): Transaction {
    // Inline import to avoid circular dependency (Transaction imports TContext from this file)
    const { Transaction: TransactionClass } = require('./transaction.ts');
    return new TransactionClass(this.inner);
  }

  // Rust: pub async fn get<R: View>(&self, id: EntityId) -> Result<R, RetrievalError>
  // Divergence: Takes ModelDefinition instead of generic R: View [E1]
  async get<V extends ViewInstance>(model: ModelDefinition<V>, id: EntityId): Promise<V> {
    const entity = await this.inner.getEntity(id, model.collection(), false);
    return model.View.fromEntity(entity);
  }

  // Rust: pub async fn get_cached<R: View>(&self, id: EntityId) -> Result<R, RetrievalError>
  async getCached<V extends ViewInstance>(model: ModelDefinition<V>, id: EntityId): Promise<V> {
    const entity = await this.inner.getEntity(id, model.collection(), true);
    return model.View.fromEntity(entity);
  }

  // Rust: pub async fn fetch<R: View>(&self, args: impl TryInto<MatchArgs>) -> Result<Vec<R>, RetrievalError>
  // Divergence: Takes ModelDefinition + MatchArgs explicitly [E1]
  async fetch<V extends ViewInstance>(model: ModelDefinition<V>, args: MatchArgs): Promise<V[]> {
    const entities = await this.inner.fetchEntities(model.collection(), args);
    return entities.map(e => model.View.fromEntity(e));
  }

  // Rust: pub async fn fetch_one<R: View>(&self, args: impl TryInto<MatchArgs>) -> Result<Option<R>, RetrievalError>
  async fetchOne<V extends ViewInstance>(model: ModelDefinition<V>, args: MatchArgs): Promise<V | null> {
    const views = await this.fetch(model, args);
    return views.length > 0 ? views[0] : null;
  }

  // Rust: pub fn query<R: View>(&self, args: impl TryInto<MatchArgs>) -> Result<LiveQuery<R>, RetrievalError>
  query<V extends ViewInstance>(model: ModelDefinition<V>, args: MatchArgs): LiveQuery<V> {
    const entityLiveQuery = this.inner.query(model.collection(), args);
    return entityLiveQuery.map(model.View);
  }

  // Rust: pub async fn query_wait<R: View>(&self, args: impl TryInto<MatchArgs>) -> Result<LiveQuery<R>, RetrievalError>
  async queryWait<V extends ViewInstance>(model: ModelDefinition<V>, args: MatchArgs): Promise<LiveQuery<V>> {
    const livequery = this.query(model, args);
    await livequery.waitInitialized();
    return livequery;
  }

  // Rust: pub async fn collection(&self, id: &CollectionId) -> Result<StorageCollectionWrapper, RetrievalError>
  // Divergence: Returns StorageCollection directly instead of StorageCollectionWrapper [E7]
  async collection(id: CollectionId): Promise<StorageCollection> {
    return this.inner.collection(id);
  }

  // Get the underlying TContext (for internal use)
  get context(): TContext {
    return this.inner;
  }
}

// ── NodeAndContext ────────────────────────────────────────────────────────────
// Rust: pub struct NodeAndContext<SE, PA: PolicyAgent> { pub node: Node<SE, PA>, pub cdata: PA::ContextData }
// Divergence: Defined in node.ts — TS Node is not generic, so NodeAndContext sits with Node [E7].
// Rust: impl TContext for NodeAndContext<SE, PA> — also in node.ts.
