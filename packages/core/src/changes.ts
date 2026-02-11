// MIRRORS: ankurah/core/src/changes.rs

import type { Attested, Event } from '@ankurah/proto';
import type { Entity } from './entity.ts';
import { MutationError } from './error.ts';

// ---------------------------------------------------------------------------
// EntityChange
// ---------------------------------------------------------------------------

/**
 * Result of committing transaction changes for a single entity.
 *
 * Rust: `pub struct EntityChange { entity: Entity, events: Vec<Attested<Event>> }`
 *
 * Validates that all events belong to the entity and are in its head clock.
 */
export class EntityChange {
  readonly entity: Entity;
  readonly events: ReadonlyArray<Attested<Event>>;

  private constructor(entity: Entity, events: Attested<Event>[]) {
    this.entity = entity;
    this.events = events;
  }

  /**
   * Create a validated EntityChange.
   *
   * Rust: `pub fn new(entity: Entity, events: Vec<Attested<Event>>) -> Result<Self, MutationError>`
   * Validates that all events have the same entity id and are in the entity's head clock.
   */
  static create(entity: Entity, events: Attested<Event>[]): EntityChange {
    const head = entity.head();
    for (const event of events) {
      if (!event.payload.entityId.equals(entity.id())) {
        throw MutationError.invalidEvent();
      }
      if (!head.contains(event.payload.id())) {
        throw MutationError.invalidEvent();
      }
    }
    return new EntityChange(entity, events);
  }

  /**
   * Decompose into parts.
   *
   * Rust: `pub fn into_parts(self) -> (Entity, Vec<Attested<Event>>)`
   */
  intoParts(): [Entity, Attested<Event>[]] {
    return [this.entity, [...this.events]];
  }

  toString(): string {
    return `EntityChange ${this.entity.collection()}/${this.entity.id()}`;
  }
}

// ---------------------------------------------------------------------------
// ChangeKind
// ---------------------------------------------------------------------------

/**
 * Classification of change type.
 *
 * Rust: `pub enum ChangeKind { Initial, Add, Remove, Update }`
 */
export type ChangeKind = 'Initial' | 'Add' | 'Remove' | 'Update';

// ---------------------------------------------------------------------------
// ItemChange<I>
// ---------------------------------------------------------------------------

/**
 * Change notification for subscription updates.
 *
 * Rust: `pub enum ItemChange<I> { Initial { item }, Add { item, events }, Update { item, events }, Remove { item, events } }`
 */
export type ItemChange<I> =
  | { readonly kind: 'Initial'; readonly item: I }
  | { readonly kind: 'Add'; readonly item: I; readonly events: ReadonlyArray<Attested<Event>> }
  | { readonly kind: 'Update'; readonly item: I; readonly events: ReadonlyArray<Attested<Event>> }
  | { readonly kind: 'Remove'; readonly item: I; readonly events: ReadonlyArray<Attested<Event>> };

/**
 * Get the item from any ItemChange variant.
 *
 * Rust: `pub fn entity(&self) -> &I`
 */
export function itemChangeItem<I>(change: ItemChange<I>): I {
  return change.item;
}

/**
 * Get the events from an ItemChange (empty array for Initial).
 *
 * Rust: `pub fn events(&self) -> &[Attested<Event>]`
 */
export function itemChangeEvents<I>(change: ItemChange<I>): ReadonlyArray<Attested<Event>> {
  if (change.kind === 'Initial') {
    return [];
  }
  return change.events;
}

/**
 * Get the ChangeKind from an ItemChange.
 *
 * Rust: `pub fn kind(&self) -> ChangeKind`
 */
export function itemChangeKind<I>(change: ItemChange<I>): ChangeKind {
  return change.kind;
}
