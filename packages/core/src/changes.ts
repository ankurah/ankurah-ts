// MIRRORS: ankurah/core/src/changes.rs

import type { Attested, Event } from '@ankurah/proto';
import type { Entity } from './entity.ts';
import { MutationError } from './error.ts';
import type { EntityResultSet } from './resultset.ts';
import type { ViewInstance, ViewConstructor } from './model.ts';

// ─── EntityChange ────────────────────────────────────────────────────────────

export class EntityChange {
  readonly entity: Entity;
  readonly events: ReadonlyArray<Attested<Event>>;

  private constructor(entity: Entity, events: Attested<Event>[]) {
    this.entity = entity;
    this.events = events;
  }

  // impl ChangeNotification for EntityChange
  intoParts(): [Entity, Attested<Event>[]] {
    return [this.entity, [...this.events]];
  }

  // Rust: pub fn new(...) -> Result<Self, MutationError>
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

  // impl Display for EntityChange
  toString(): string {
    return `EntityChange ${this.entity.collection()}/${this.entity.id()}`;
  }
}

// ─── ItemChange<I> ───────────────────────────────────────────────────────────

// Divergence: Rust enum ItemChange<I> → TS discriminated union [E8]
// Cannot use Enum<V> due to generic type parameter.
export type ItemChange<I> =
  | { readonly kind: 'Initial'; readonly item: I }
  | { readonly kind: 'Add'; readonly item: I; readonly events: ReadonlyArray<Attested<Event>> }
  | { readonly kind: 'Update'; readonly item: I; readonly events: ReadonlyArray<Attested<Event>> }
  | { readonly kind: 'Remove'; readonly item: I; readonly events: ReadonlyArray<Attested<Event>> };

// impl<I> ItemChange<I> { pub fn entity(&self) -> &I }
export function itemChangeItem<I>(change: ItemChange<I>): I {
  return change.item;
}

// impl<I> ItemChange<I> { pub fn events(&self) -> &[Attested<Event>] }
export function itemChangeEvents<I>(change: ItemChange<I>): ReadonlyArray<Attested<Event>> {
  if (change.kind === 'Initial') {
    return [];
  }
  return change.events;
}

// impl<I> ItemChange<I> { pub fn kind(&self) -> ChangeKind }
export function itemChangeKind<I>(change: ItemChange<I>): ChangeKind {
  return change.kind;
}

// impl<I> Display for ItemChange<I> where I: View
export function itemChangeToString<I extends ViewInstance>(change: ItemChange<I>, _ctor: ViewConstructor<I>): string {
  const collection = change.item.collection();
  switch (change.kind) {
    case 'Initial': return `Initial ${collection}/${change.item.id()}`;
    case 'Add': return `Add ${collection}/${change.item.id()}`;
    case 'Update': return `Update ${collection}/${change.item.id()}`;
    case 'Remove': return `Remove ${collection}/${change.item.id()}`;
  }
}

// ─── ChangeSet<V> ────────────────────────────────────────────────────────────

export class ChangeSet<V extends ViewInstance> {
  readonly resultset: EntityResultSet;
  readonly changes: ReadonlyArray<ItemChange<V>>;

  constructor(resultset: EntityResultSet, changes: ItemChange<V>[]) {
    this.resultset = resultset;
    this.changes = changes;
  }

  /// Returns items from the initial query load (before subscription was active)
  initial(): V[] {
    return this.changes
      .filter((c): c is ItemChange<V> & { kind: 'Initial' } => c.kind === 'Initial')
      .map((c) => c.item);
  }

  /// Returns genuinely new items (added after subscription, or now match the predicate)
  added(): V[] {
    return this.changes
      .filter((c): c is ItemChange<V> & { kind: 'Add' } => c.kind === 'Add')
      .map((c) => c.item);
  }

  /// Returns all items that appeared in the result set (initial load + newly added)
  appeared(): V[] {
    return this.changes
      .filter((c) => c.kind === 'Initial' || c.kind === 'Add')
      .map((c) => c.item);
  }

  /** @deprecated Use `appeared()`, `initial()`, or `added()` instead */
  adds(): V[] { return this.appeared(); }

  /// Returns all items that were removed or no longer match the query
  removed(): V[] {
    return this.changes
      .filter((c): c is ItemChange<V> & { kind: 'Remove' } => c.kind === 'Remove')
      .map((c) => c.item);
  }

  /** @deprecated Use `removed()` instead */
  removes(): V[] { return this.removed(); }

  /// Returns all items that were updated but still match the query
  updated(): V[] {
    return this.changes
      .filter((c): c is ItemChange<V> & { kind: 'Update' } => c.kind === 'Update')
      .map((c) => c.item);
  }

  /** @deprecated Use `updated()` instead */
  updates(): V[] { return this.updated(); }

  // impl Display for ChangeSet<I> where I: View + Clone
  toStringWith(ctor: ViewConstructor<V>): string {
    const results = this.resultset.len();
    const changesStr = this.changes.map((c) => itemChangeToString(c, ctor)).join(', ');
    return `ChangeSet(${results} results): ${changesStr}`;
  }
}

// impl<I> From<ItemChange<Entity>> for ItemChange<I> where I: View
export function itemChangeFromEntity<I extends ViewInstance>(
  change: ItemChange<Entity>,
  fromEntity: (entity: Entity) => I,
): ItemChange<I> {
  switch (change.kind) {
    case 'Initial':
      return { kind: 'Initial', item: fromEntity(change.item) };
    case 'Add':
      return { kind: 'Add', item: fromEntity(change.item), events: change.events };
    case 'Update':
      return { kind: 'Update', item: fromEntity(change.item), events: change.events };
    case 'Remove':
      return { kind: 'Remove', item: fromEntity(change.item), events: change.events };
  }
}

// ─── ChangeKind ──────────────────────────────────────────────────────────────

// Divergence: Rust enum ChangeKind → TS string literal union [E8]
export type ChangeKind = 'Initial' | 'Add' | 'Remove' | 'Update';
