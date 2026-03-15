// MIRRORS: ankurah/core/src/transaction.rs
import { TransactionId, type CollectionId, type EntityId } from '@ankurah/proto';
import { disposeSymbol } from '@ankurah/base';
import type { TContext } from './context.ts';
import type { Entity } from './entity.ts';
import { MutationError } from './error.ts';
import type { ModelDefinition, MutableInstance, ViewInstance } from './model.ts';
import { MutableBorrow } from './model.ts';

// Q. When do we want unified vs individual property storage for TypeEngine operations?
// A. When we start to care about differentiating possible recipients for different properties.

// ── Transaction ──────────────────────────────────────────────────────────────
// Rust: pub struct Transaction
// Divergence: Rust uses AppendOnlyVec for entities — TS uses plain array [E8].
// Divergence: Rust uses Arc<AtomicBool> for alive — TS uses { value: boolean } [E8].
// Divergence: Rust uses RwLock<HashSet<EntityId>> for created_entity_ids — TS uses plain Set [E8].

export class Transaction {
  readonly dyncontext: TContext;
  readonly id: TransactionId;
  readonly entities: Entity[];
  readonly alive: { value: boolean };
  // Entity IDs that were created in this transaction via create().
  // Used to validate that creation events (empty parent) are only for entities
  // that were actually created in this transaction, not phantom entities.
  readonly createdEntityIds: Set<string>;

  // Rust: pub(crate) fn new(dyncontext: Arc<dyn TContext + Send + Sync + 'static>) -> Self
  constructor(dyncontext: TContext) {
    this.dyncontext = dyncontext;
    this.id = TransactionId.new();
    this.entities = [];
    this.alive = { value: true };
    this.createdEntityIds = new Set();
  }

  // Rust: pub(crate) fn add_entity(&self, entity: Entity) -> &Entity
  private addEntity(entity: Entity): Entity {
    this.entities.push(entity);
    return entity;
  }

  // Rust: pub async fn create<'rec, 'trx: 'rec, M: Model>(&'trx self, model: &M) -> Result<MutableBorrow<'rec, M::Mutable>, MutationError>
  // Divergence: Takes ModelDefinition + values instead of Model instance [E1].
  async create<V extends ViewInstance, M extends MutableInstance>(
    model: ModelDefinition<V, M>,
    values: Record<string, unknown> = {},
  ): Promise<MutableBorrow<M>> {
    if (!this.alive.value) {
      throw new MutationError('General', 'Transaction has been consumed');
    }
    const entity = this.dyncontext.createEntity(model.collection(), this.alive);
    model.initializeNewEntity(entity, values);
    this.dyncontext.checkWrite(entity); // Divergence: Rust returns Result; TS throws [E3]

    // Track that this entity was created in this transaction
    this.createdEntityIds.add(entity.id().toString());

    const entityRef = this.addEntity(entity);
    return new MutableBorrow(entityRef, model.Mutable);
  }

  // Rust: fn get_trx_entity(&self, id: &EntityId) -> Option<&Entity>
  private getTrxEntity(id: EntityId): Entity | null {
    for (const entity of this.entities) {
      if (entity.id().equals(id)) {
        return entity;
      }
    }
    return null;
  }

  // Rust: pub async fn get<'rec, 'trx: 'rec, M: Model>(&'trx self, id: &EntityId) -> Result<MutableBorrow<'rec, M::Mutable>, RetrievalError>
  async get<V extends ViewInstance, M extends MutableInstance>(
    model: ModelDefinition<V, M>,
    id: EntityId,
  ): Promise<MutableBorrow<M>> {
    if (!this.alive.value) {
      throw new MutationError('General', 'Transaction has been consumed');
    }
    const existing = this.getTrxEntity(id);
    if (existing) {
      return new MutableBorrow(existing, model.Mutable);
    }

    // Go fetch the entity from the context
    const retrievedEntity = await this.dyncontext.getEntity(id, model.collection(), false);

    // Double check to make sure somebody didn't add the entity to the trx during the await
    // because we're forking the entity, we need to make sure we aren't adding the same entity twice
    const raceCheck = this.getTrxEntity(retrievedEntity.id());
    if (raceCheck) {
      // If this happens, we don't want to refresh the entity, because it's already snapshotted
      // in the trx and we should leave it that way to honor the consistency model
      return new MutableBorrow(raceCheck, model.Mutable);
    }

    return new MutableBorrow(this.addEntity(retrievedEntity.snapshot(this.alive)), model.Mutable);
  }

  // Rust: pub fn edit<'rec, 'trx: 'rec, M: Model>(&'trx self, entity: &Entity) -> Result<MutableBorrow<'rec, M::Mutable>, AccessDenied>
  edit<V extends ViewInstance, M extends MutableInstance>(
    model: ModelDefinition<V, M>,
    entity: Entity,
  ): MutableBorrow<M> {
    const existing = this.getTrxEntity(entity.id());
    if (existing) {
      return new MutableBorrow(existing, model.Mutable);
    }
    this.dyncontext.checkWrite(entity); // Divergence: Rust returns Result; TS throws [E3]

    return new MutableBorrow(this.addEntity(entity.snapshot(this.alive)), model.Mutable);
  }

  // Rust: #[must_use] pub async fn commit(self) -> Result<(), MutationError>
  async commit(): Promise<void> {
    return this.dyncontext.commitLocalTrx(this);
  }

  // Rust: pub fn rollback(self)
  rollback(): void {
    // Mark transaction as no longer alive
    this.alive.value = false;
    // The transaction will be dropped without committing
  }

  // TODO: Implement delete functionality after core query/edit operations are stable
  // For now, "removal" from result sets is handled by edits that cause entities to no longer match queries

  // Rust: impl Drop for Transaction
  // Divergence: Symbol.dispose instead of Drop trait [E11].
  [disposeSymbol](): void {
    // Mark transaction as no longer alive when dropped
    this.alive.value = false;
  }
}
