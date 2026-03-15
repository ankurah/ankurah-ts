// MIRRORS: ankurah/core/src/reactor/update.rs

import type { QueryId, Attested, Event } from '@ankurah/proto';
import type { Entity } from '../entity.ts';

// ---------------------------------------------------------------------------
// MembershipChange
// ---------------------------------------------------------------------------

/**
 * Describes how an entity's membership changed for a specific predicate.
 *
 * Rust: `pub enum MembershipChange { Initial, Add, Remove }`
 * Divergence: Unit-only enum → string union (no data variants, no Drop needed) [E8]
 */
export type MembershipChange = 'Initial' | 'Add' | 'Remove';

// ---------------------------------------------------------------------------
// ReactorUpdate
// ---------------------------------------------------------------------------

/**
 * Update from the reactor that supports both single and multi-predicate subscriptions.
 *
 * Rust: `pub struct ReactorUpdate<E = Entity, Ev = Attested<Event>>`
 * Divergence: Rust generics exist only for testing with mock types; TS uses concrete types directly.
 */
export interface ReactorUpdate {
  /** All entities that changed, with their relevance information. */
  items: ReactorUpdateItem[];
}

// ---------------------------------------------------------------------------
// ReactorUpdateItem
// ---------------------------------------------------------------------------

/**
 * A single entity update with all relevance information.
 *
 * Rust: `pub struct ReactorUpdateItem<E = Entity, Ev = Attested<Event>>`
 * Divergence: Rust generics exist only for testing with mock types; TS uses concrete types directly.
 */
export interface ReactorUpdateItem {
  /** The entity that changed. */
  entity: Entity;

  /** Events that caused this update. */
  events: Attested<Event>[];

  /**
   * Which predicates this update is relevant to (if any) and how.
   * Rust: `pub predicate_relevance: Vec<(QueryId, MembershipChange)>`
   */
  predicateRelevance: [QueryId, MembershipChange][];
}

// ---------------------------------------------------------------------------
// impl ReactorUpdateItem
// ---------------------------------------------------------------------------

/**
 * Check if this item represents any membership change.
 *
 * Rust: `impl ReactorUpdateItem { pub fn has_membership_change(&self) -> bool }`
 */
export function hasMembershipChange(item: ReactorUpdateItem): boolean {
  return item.predicateRelevance.length > 0;
}

// Note: ReactorUpdate to ChangeSet<Entity> conversion removed since Entity doesn't implement View
// ReactorUpdate should be converted to ChangeSet<R> at the LiveQuery level instead
