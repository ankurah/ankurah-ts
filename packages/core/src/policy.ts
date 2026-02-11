// MIRRORS: ankurah/core/src/policy.rs

import type { CollectionId, Attestation, EntityState, Event } from '@ankurah/proto';
import type { Entity } from './entity.ts';
import type { AccessDenied } from './error.ts';

// ---------------------------------------------------------------------------
// PolicyAgent — trait for access control policy implementations
// ---------------------------------------------------------------------------

/**
 * Interface for policy agent implementations that control access to entities.
 *
 * Rust: `pub trait PolicyAgent: Send + Sync + 'static`
 *
 * The policy agent is generic over ContextData (authentication/authorization context).
 * It validates reads, writes, and event attestation.
 *
 * Divergence: Rust uses associated type `type ContextData: ContextData`; TS uses
 * generic parameter [A6].
 */
export interface PolicyAgent<ContextData = unknown> {
  /**
   * Check if the given entity can be written to.
   *
   * Rust: `fn check_write(&self, cdata: &CD, entity: &Entity, event: Option<&Event>) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  checkWrite(cdata: ContextData, entity: Entity, event: Event | null): void;

  /**
   * Check if a collection can be accessed.
   *
   * Rust: `fn can_access_collection(&self, cdata: &CD, collection: &CollectionId) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  canAccessCollection(cdata: ContextData, collection: CollectionId): void;

  /**
   * Check an event against policy and optionally return an attestation.
   *
   * Rust: `fn check_event(&self, node: &Node, cdata: &CD, before: &Entity, after: &Entity, event: &Event) -> Result<Option<Attestation>, AccessDenied>`
   */
  checkEvent(
    cdata: ContextData,
    entityBefore: Entity,
    entityAfter: Entity,
    event: Event,
  ): Attestation | null;

  /**
   * Create an attestation for a state snapshot.
   *
   * Rust: `fn attest_state(&self, node: &Node, state: &EntityState) -> Option<Attestation>`
   */
  attestState(state: EntityState): Attestation | null;

  /**
   * Filter predicate for collection access control.
   *
   * Rust: `fn filter_predicate(&self, cdata: &CD, collection: &CollectionId, predicate: Predicate) -> Result<Predicate, AccessDenied>`
   */
  filterPredicate?(
    cdata: ContextData,
    collection: CollectionId,
    predicate: unknown,
  ): unknown;
}

// ---------------------------------------------------------------------------
// OpenPolicy — a policy agent that allows everything
// ---------------------------------------------------------------------------

/**
 * A policy agent that allows all operations (no restrictions).
 * Useful for testing and development.
 *
 * Rust: Similar to `impl PolicyAgent for ()` or default implementations.
 */
export class OpenPolicy implements PolicyAgent<unknown> {
  checkWrite(): void {
    // Allow all writes
  }

  canAccessCollection(): void {
    // Allow all collection access
  }

  checkEvent(): Attestation | null {
    return null; // No attestation
  }

  attestState(): Attestation | null {
    return null; // No attestation
  }
}
