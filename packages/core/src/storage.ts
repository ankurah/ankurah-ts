// MIRRORS: ankurah/core/src/storage.rs

import type { Attested, CollectionId, EntityId, EntityState, Event, EventId } from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';

import { MutationError, RetrievalError } from './error.ts';

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/** Rust: `pub fn state_name(name: &str) -> String` */
export function stateName(name: string): string {
  return `${name}_state`;
}

/** Rust: `pub fn event_name(name: &str) -> String` */
export function eventName(name: string): string {
  return `${name}_event`;
}

// ---------------------------------------------------------------------------
// StorageEngine — trait for storage engine implementations
// ---------------------------------------------------------------------------

/**
 * Interface for storage engine implementations.
 *
 * Rust: `pub trait StorageEngine: Send + Sync`
 * Divergence: Rust has associated type `type Value`; omitted in TS — unused by trait methods [E8].
 */
export interface StorageEngine {
  /**
   * Opens and/or creates a storage collection.
   *
   * Rust: `async fn collection(&self, id: &CollectionId) -> Result<Arc<dyn StorageCollection>, RetrievalError>`
   */
  collection(id: CollectionId): Promise<StorageCollection>;

  /**
   * Delete all collections and their data from the storage engine.
   *
   * Rust: `async fn delete_all_collections(&self) -> Result<bool, MutationError>`
   */
  deleteAllCollections(): Promise<boolean>;
}

// ---------------------------------------------------------------------------
// StorageCollection — trait for collection-level storage operations
// ---------------------------------------------------------------------------

/**
 * Interface for collection-level storage operations.
 *
 * Rust: `pub trait StorageCollection: Send + Sync`
 * All methods are async (matching Rust async_trait).
 */
export interface StorageCollection {
  /**
   * Set/update the state for an entity.
   *
   * Rust: `async fn set_state(&self, state: Attested<EntityState>) -> Result<bool, MutationError>`
   */
  setState(state: Attested<EntityState>): Promise<boolean>;

  /**
   * Get the state for a specific entity.
   *
   * Rust: `async fn get_state(&self, id: EntityId) -> Result<Attested<EntityState>, RetrievalError>`
   */
  getState(id: EntityId): Promise<Attested<EntityState>>;

  /**
   * Fetch raw entity states matching a selection (predicate + order by + limit).
   *
   * Rust: `async fn fetch_states(&self, selection: &Selection) -> Result<Vec<Attested<EntityState>>, RetrievalError>`
   */
  fetchStates(selection: Selection): Promise<Attested<EntityState>[]>;

  /**
   * Add an event to the collection's event log.
   *
   * Rust: `async fn add_event(&self, entity_event: &Attested<Event>) -> Result<bool, MutationError>`
   */
  addEvent(event: Attested<Event>): Promise<boolean>;

  /**
   * Retrieve a list of events by their IDs.
   *
   * Rust: `async fn get_events(&self, event_ids: Vec<EventId>) -> Result<Vec<Attested<Event>>, RetrievalError>`
   */
  getEvents(eventIds: EventId[]): Promise<Attested<Event>[]>;

  /**
   * Retrieve all events for an entity from the collection.
   *
   * Rust: `async fn dump_entity_events(&self, id: EntityId) -> Result<Vec<Attested<Event>>, RetrievalError>`
   */
  dumpEntityEvents(id: EntityId): Promise<Attested<Event>[]>;
}

// ---------------------------------------------------------------------------
// Default implementations for StorageCollection
// ---------------------------------------------------------------------------

/**
 * Set multiple states. Default implementation iterates and calls setState.
 *
 * Rust: `async fn set_states(&self, states: Vec<Attested<EntityState>>) -> Result<(), MutationError>`
 * Divergence: Free function instead of default trait method — TS interfaces cannot have default impls [E7].
 */
export async function setStates(collection: StorageCollection, states: Attested<EntityState>[]): Promise<void> {
  for (const state of states) {
    await collection.setState(state);
  }
}

/**
 * Get multiple states by ID. Silently skips entities that are not found.
 *
 * Rust: `async fn get_states(&self, ids: Vec<EntityId>) -> Result<Vec<Attested<EntityState>>, RetrievalError>`
 * Divergence: Free function instead of default trait method — TS interfaces cannot have default impls [E7].
 */
export async function getStates(collection: StorageCollection, ids: EntityId[]): Promise<Attested<EntityState>[]> {
  const states: Attested<EntityState>[] = [];
  for (const id of ids) {
    try {
      const state = await collection.getState(id);
      states.push(state);
    } catch (e) {
      if (e instanceof RetrievalError && e.kind === 'EntityNotFound') {
        console.warn(`Entity not found: ${id}`);
        continue;
      }
      throw e;
    }
  }
  return states;
}

// ---------------------------------------------------------------------------
// StorageCollectionWrapper
// ---------------------------------------------------------------------------

/**
 * Manages the storage and state of the collection without any knowledge of the model type.
 *
 * Rust: `pub struct StorageCollectionWrapper(pub(crate) Arc<dyn StorageCollection>)`
 * Divergence: No Deref — TS has no Deref trait. Access inner via `.inner` field [E8].
 */
export class StorageCollectionWrapper {
  readonly inner: StorageCollection;

  constructor(bucket: StorageCollection) {
    this.inner = bucket;
  }
}
