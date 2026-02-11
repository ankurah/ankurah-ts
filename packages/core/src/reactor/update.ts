// MIRRORS: ankurah/core/src/reactor/update.rs

import type { QueryId, Attested, Event } from '@ankurah/proto';
import type { Entity } from '../entity.ts';

// ---------------------------------------------------------------------------
// MembershipChange
// ---------------------------------------------------------------------------

/**
 * Whether an entity was added to, removed from, or initially present in a query result set.
 *
 * Rust: `pub enum MembershipChange { Initial, Add, Remove }`
 */
export type MembershipChange = 'Initial' | 'Add' | 'Remove';

// ---------------------------------------------------------------------------
// ReactorUpdateItem
// ---------------------------------------------------------------------------

/**
 * A single entity update within a reactor batch.
 *
 * Rust: `pub struct ReactorUpdateItem<E = Entity, Ev = Attested<Event>>`
 * Divergence: Rust generics exist only for testing with mock types; TS uses concrete types directly.
 */
export interface ReactorUpdateItem {
  /** The entity that was updated. */
  entity: Entity;

  /** Events that triggered this update. */
  events: Attested<Event>[];

  /**
   * Which queries this entity is relevant to, and how its membership changed.
   * Rust: `pub predicate_relevance: Vec<(QueryId, MembershipChange)>`
   */
  predicateRelevance: [QueryId, MembershipChange][];
}

/**
 * Whether this item has any membership changes (add/remove/initial).
 *
 * Rust: `impl ReactorUpdateItem { pub fn has_membership_change(&self) -> bool }`
 */
export function hasMembershipChange(item: ReactorUpdateItem): boolean {
  return item.predicateRelevance.length > 0;
}

// ---------------------------------------------------------------------------
// ReactorUpdate
// ---------------------------------------------------------------------------

/**
 * A batch of entity updates to be processed by the reactor.
 *
 * Rust: `pub struct ReactorUpdate<E = Entity, Ev = Attested<Event>>`
 * Divergence: Rust generics exist only for testing with mock types; TS uses concrete types directly.
 */
export interface ReactorUpdate {
  /** The items in this update batch. */
  items: ReactorUpdateItem[];
}
