// MIRRORS: ankurah/core/src/transaction.rs

import { TransactionId, type CollectionId, type EntityId } from '@ankurah/proto';
import type { TContext } from './context.ts';
import type { Entity } from './entity.ts';
import { MutationError, type RetrievalError } from './error.ts';
import type { AccessDenied } from './error.ts';
import type { ModelDefinition, MutableInstance, ViewInstance, MutableConstructor } from './model.ts';
import { MutableBorrow } from './model.ts';

// ---------------------------------------------------------------------------
// Transaction
// ---------------------------------------------------------------------------

/**
 * A transaction groups entity mutations and commits them atomically.
 *
 * Rust: `pub struct Transaction { dyncontext, id, entities, alive, created_entity_ids }`
 *
 * Divergence: Rust uses AppendOnlyVec for entities — TS uses plain array [E8].
 * Divergence: Rust uses Arc<AtomicBool> for alive — TS uses { value: boolean } [E8].
 * Divergence: Rust uses RwLock<HashSet<EntityId>> — TS uses plain Set [E8].
 * Divergence: Rust lifetime constraints ('rec, 'trx) prevent MutableBorrow from
 *   outliving the transaction at compile time — TS has no lifetimes, so this is
 *   a runtime check via the alive flag.
 */
export class Transaction {
  /** The context implementation backing this transaction */
  readonly dyncontext: TContext;

  /** Unique transaction identifier */
  readonly id: TransactionId;

  /**
   * Entities that have been forked into this transaction.
   *
   * Rust: `pub(crate) entities: AppendOnlyVec<Entity>`
   * TS: Plain array (single-threaded JS, no need for lock-free append-only).
   */
  readonly entities: Entity[];

  /**
   * Shared alive flag — when false, all forked entities become non-writable.
   *
   * Rust: `pub(crate) alive: Arc<AtomicBool>`
   * TS: Shared reference object { value: boolean } [E8].
   */
  readonly alive: { value: boolean };

  /**
   * Entity IDs that were created in this transaction via create().
   * Used to validate that creation events (empty parent) are only for entities
   * that were actually created in this transaction, not phantom entities.
   *
   * Rust: `pub(crate) created_entity_ids: RwLock<HashSet<EntityId>>`
   * TS: Plain Set (single-threaded JS).
   */
  readonly createdEntityIds: Set<string>;

  constructor(dyncontext: TContext) {
    this.dyncontext = dyncontext;
    this.id = TransactionId.new();
    this.entities = [];
    this.alive = { value: true };
    this.createdEntityIds = new Set();
  }

  // ── Internal helpers ──────────────────────────────────────────────────

  /**
   * Add an entity to this transaction's entity list.
   *
   * Rust: `fn add_entity(&self, entity: Entity) -> &Entity`
   */
  private addEntity(entity: Entity): Entity {
    this.entities.push(entity);
    return entity;
  }

  /**
   * Find an entity in this transaction by ID.
   *
   * Rust: `fn get_trx_entity(&self, id: &EntityId) -> Option<&Entity>`
   */
  private getTrxEntity(id: EntityId): Entity | null {
    for (const entity of this.entities) {
      if (entity.id().equals(id)) {
        return entity;
      }
    }
    return null;
  }

  // ── Entity lifecycle ──────────────────────────────────────────────────

  /**
   * Create a new entity within this transaction.
   *
   * Rust: `pub async fn create<'rec, 'trx: 'rec, M: Model>(&'trx self, model: &M) -> Result<MutableBorrow<'rec, M::Mutable>, MutationError>`
   *
   * Process:
   * 1. Context creates primary entity + transacted fork
   * 2. Model initializes backends with field values
   * 3. Policy check_write validates access
   * 4. Track entity ID in created set
   * 5. Add to transaction entity list
   * 6. Return MutableBorrow wrapping the mutable accessor
   *
   * @param model - The model definition (from defineModel)
   * @param values - Initial field values to set on the entity
   */
  async create<V extends ViewInstance, M extends MutableInstance>(
    model: ModelDefinition<V, M>,
    values: Record<string, unknown> = {},
  ): Promise<MutableBorrow<M>> {
    const entity = this.dyncontext.createEntity(model.collection(), this.alive);
    model.initializeNewEntity(entity, values);
    this.dyncontext.checkWrite(entity);

    // Track that this entity was created in this transaction
    this.createdEntityIds.add(entity.id().toString());

    const addedEntity = this.addEntity(entity);
    return new MutableBorrow(addedEntity, model.Mutable);
  }

  /**
   * Get an entity by ID within this transaction (fetch from storage if needed).
   *
   * Rust: `pub async fn get<'rec, 'trx: 'rec, M: Model>(&'trx self, id: &EntityId) -> Result<MutableBorrow<'rec, M::Mutable>, RetrievalError>`
   *
   * Process:
   * 1. Check if entity already in transaction → return existing fork
   * 2. Fetch from context (storage/peers)
   * 3. Race check: re-examine transaction list
   * 4. Fork the retrieved entity into this transaction
   * 5. Return MutableBorrow
   */
  async get<V extends ViewInstance, M extends MutableInstance>(
    model: ModelDefinition<V, M>,
    id: EntityId,
  ): Promise<MutableBorrow<M>> {
    // Check transaction-local first
    const existing = this.getTrxEntity(id);
    if (existing) {
      return new MutableBorrow(existing, model.Mutable);
    }

    // Fetch from context
    const retrievedEntity = await this.dyncontext.getEntity(id, model.collection(), false);

    // Race check: another async path might have added it
    const raceCheck = this.getTrxEntity(retrievedEntity.id());
    if (raceCheck) {
      return new MutableBorrow(raceCheck, model.Mutable);
    }

    // Fork into this transaction
    const forked = this.addEntity(retrievedEntity.snapshot(this.alive));
    return new MutableBorrow(forked, model.Mutable);
  }

  /**
   * Edit an entity that's already loaded (fork into transaction if needed).
   *
   * Rust: `pub fn edit<'rec, 'trx: 'rec, M: Model>(&'trx self, entity: &Entity) -> Result<MutableBorrow<'rec, M::Mutable>, AccessDenied>`
   *
   * Process:
   * 1. Check if already in transaction → return existing fork
   * 2. Policy check_write validates access
   * 3. Fork entity into this transaction
   * 4. Return MutableBorrow
   */
  edit<V extends ViewInstance, M extends MutableInstance>(
    model: ModelDefinition<V, M>,
    entity: Entity,
  ): MutableBorrow<M> {
    // Check transaction-local first
    const existing = this.getTrxEntity(entity.id());
    if (existing) {
      return new MutableBorrow(existing, model.Mutable);
    }

    this.dyncontext.checkWrite(entity);
    const forked = this.addEntity(entity.snapshot(this.alive));
    return new MutableBorrow(forked, model.Mutable);
  }

  // ── Commit / Rollback ─────────────────────────────────────────────────

  /**
   * Commit the transaction, persisting all mutations.
   *
   * Rust: `pub async fn commit(self) -> Result<(), MutationError>`
   *
   * Delegates to the context's commit_local_trx implementation (the full
   * 7-phase commit pipeline: alive check, event generation, policy validation,
   * attestation, storage, replication, notification).
   *
   * This method "consumes" the transaction: after commit, the alive flag is
   * false and all forked entities become non-writable.
   */
  async commit(): Promise<void> {
    return this.dyncontext.commitLocalTrx(this);
  }

  /**
   * Rollback the transaction, discarding all mutations.
   *
   * Rust: `pub fn rollback(self)`
   *
   * Marks the alive flag as false, making all forked entities non-writable.
   * No changes are persisted.
   */
  rollback(): void {
    this.alive.value = false;
  }
}
