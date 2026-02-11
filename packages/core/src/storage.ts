// MIRRORS: ankurah/core/src/storage.rs

import type { CollectionId, EntityId, EventId, Attested, EntityState, Event } from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';
import type { RetrievalError, StateError } from './error.ts';

// ---------------------------------------------------------------------------
// StorageCollection — trait for collection-level storage operations
// ---------------------------------------------------------------------------

/**
 * Interface for collection-level storage operations.
 *
 * Rust: `pub trait StorageCollection: Send + Sync`
 *
 * Implementations: SQLite (expo-sqlite, better-sqlite3), Memory
 * All methods are async (matching Rust async_trait).
 */
export interface StorageCollection {
  /**
   * Get the state for a specific entity.
   *
   * Rust: `async fn get_state(&self, id: EntityId) -> Result<Attested<EntityState>, RetrievalError>`
   */
  getState(id: EntityId): Promise<Attested<EntityState>>;

  /**
   * Set/update the state for an entity.
   *
   * Rust: `async fn set_state(&self, state: Attested<EntityState>) -> Result<(), StateError>`
   */
  setState(state: Attested<EntityState>): Promise<void>;

  /**
   * Add an event to the collection's event log.
   *
   * Rust: `async fn add_event(&self, event: &Attested<Event>) -> Result<(), MutationError>`
   */
  addEvent(event: Attested<Event>): Promise<void>;

  /**
   * Retrieve a list of events by their IDs.
   *
   * Rust: `async fn get_events(&self, event_ids: Vec<EventId>) -> Result<Vec<Attested<Event>>, RetrievalError>`
   */
  getEvents(eventIds: EventId[]): Promise<Attested<Event>[]>;

  /**
   * Fetch entity states matching a selection predicate.
   *
   * Rust: `async fn fetch_states(&self, selection: &Selection) -> Result<Vec<Attested<EntityState>>, RetrievalError>`
   */
  fetchStates(selection: Selection): Promise<Attested<EntityState>[]>;
}

// ---------------------------------------------------------------------------
// StorageEngine — trait for storage engine implementations
// ---------------------------------------------------------------------------

/**
 * Interface for storage engine implementations.
 *
 * Rust: `pub trait StorageEngine: Send + Sync + 'static`
 *
 * The storage engine manages collections and provides access to them.
 * Implementations: SQLite, Memory, etc.
 */
export interface StorageEngine {
  /**
   * Get or create a storage collection for the given collection ID.
   *
   * Rust: `async fn collection(&self, id: &CollectionId) -> Result<StorageCollectionWrapper, RetrievalError>`
   */
  collection(id: CollectionId): Promise<StorageCollection>;
}
