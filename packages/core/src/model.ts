// MIRRORS: ankurah/core/src/model.rs

import type { CollectionId, EntityId, State } from '@ankurah/proto';
import type { PropertyError } from './property/traits.ts';
import type { StateError } from './error.ts';
import type { Entity } from './entity.ts';

// ---------------------------------------------------------------------------
// Model trait interface
// ---------------------------------------------------------------------------

/**
 * A model is a struct that represents the present values for a given entity.
 * Schema is defined primarily by the Model object, and the View is derived from that via macro.
 *
 * Rust: `pub trait Model: Sized { type View: View; type Mutable: Mutable; ... }`
 * TS: Interface, with associated types expressed as generic parameters.
 *
 * Exception E1: Rust uses #[derive(Model)] proc macro to generate View/Mutable impls.
 * In TS, defineModel() generates these at runtime (see define-model.ts).
 * Exception E9: WASM-only RefWrapper associated type is omitted.
 */
export interface ModelDefinition<V extends ViewInstance = ViewInstance, M extends MutableInstance = MutableInstance> {
  /** The View class/constructor for this model */
  readonly View: ViewConstructor<V>;

  /** The Mutable class/constructor for this model */
  readonly Mutable: MutableConstructor<M>;

  /**
   * Get the collection identifier for this model.
   *
   * Rust: `fn collection() -> CollectionId`
   */
  collection(): CollectionId;

  /**
   * Initialize a new entity with the given field values.
   *
   * Rust: `fn initialize_new_entity(&self, entity: &Entity)`
   * TS: Takes a values record instead of `self` because TS models are not struct instances.
   */
  initializeNewEntity(entity: Entity, values: Record<string, unknown>): void;
}

// ---------------------------------------------------------------------------
// View trait interface
// ---------------------------------------------------------------------------

/**
 * A read-only view of an Entity which offers typed accessors.
 *
 * Rust: `pub trait View { type Model: Model; type Mutable: Mutable; ... }`
 * TS: Interface. Associated types removed in favor of the ModelDefinition holding both.
 *
 * Divergence: Rust View has default impls for id() and collection() that delegate;
 * TS interface cannot have default impls, so implementors must provide them [E7].
 */
export interface ViewInstance {
  /**
   * Get the entity ID for this view.
   *
   * Rust: `fn id(&self) -> EntityId { self.entity().id() }`
   */
  id(): EntityId;

  /**
   * Get the collection identifier for this model.
   *
   * Rust: `fn collection() -> CollectionId { <Self::Model as Model>::collection() }`
   */
  collection(): CollectionId;

  /**
   * Get the underlying entity.
   *
   * Rust: `fn entity(&self) -> &Entity`
   */
  entity(): Entity;
}

/**
 * Constructor interface for View types.
 *
 * Rust: `fn from_entity(inner: Entity) -> Self` is a static method on the View trait.
 * TS: Modeled as a separate constructor interface since interfaces cannot have static methods.
 */
export interface ViewConstructor<V extends ViewInstance = ViewInstance> {
  /**
   * Construct a View from an Entity.
   *
   * Rust: `fn from_entity(inner: Entity) -> Self`
   */
  fromEntity(entity: Entity): V;
}

// ---------------------------------------------------------------------------
// Mutable trait interface
// ---------------------------------------------------------------------------

/**
 * A mutable Model instance for an Entity with typed accessors.
 * It is associated with a transaction, and may not outlive said transaction.
 *
 * Rust: `pub trait Mutable { type Model: Model; type View: View; ... }`
 * TS: Interface. Associated types removed in favor of the ModelDefinition holding both.
 *
 * Divergence: Rust Mutable has default impls for id(), collection(), state(), and read();
 * TS interface cannot have default impls, so implementors must provide them [E7].
 */
export interface MutableInstance {
  /**
   * Get the entity ID for this mutable.
   *
   * Rust: `fn id(&self) -> EntityId { self.entity().id() }`
   */
  id(): EntityId;

  /**
   * Get the collection identifier for this model.
   *
   * Rust: `fn collection() -> CollectionId { <Self::Model as Model>::collection() }`
   */
  collection(): CollectionId;

  /**
   * Get the underlying entity.
   *
   * Rust: `fn entity(&self) -> &Entity`
   */
  entity(): Entity;

  /**
   * Get the current state of the entity.
   * Throws StateError on failure.
   *
   * Rust: `fn state(&self) -> Result<State, StateError> { self.entity().to_state() }`
   */
  state(): State;

  /**
   * Get a read-only view of the current mutable state.
   *
   * Rust: `fn read(&self) -> Self::View`
   */
  read(): ViewInstance;
}

/**
 * Constructor interface for Mutable types.
 *
 * Rust: `fn new(entity: Entity) -> Self` is a static method on the Mutable trait.
 * TS: Modeled as a separate constructor interface since interfaces cannot have static methods.
 */
export interface MutableConstructor<M extends MutableInstance = MutableInstance> {
  new (entity: Entity): M;
}

// ---------------------------------------------------------------------------
// MutableBorrow — lifetime-constrained wrapper
// ---------------------------------------------------------------------------

/**
 * A wrapper around a Mutable that conceptually ties it to an Entity reference.
 *
 * Rust: `pub struct MutableBorrow<'rec, T: Mutable> { mutable: T, _entity_ref: &'rec Entity }`
 * Divergence: Rust uses lifetime constraints for compile-time transaction safety;
 * TS has no lifetime system, so this is a simple wrapper class [E8].
 *
 * In Rust, MutableBorrow implements Deref/DerefMut to T. In TS, we expose the
 * inner mutable directly since TS has no Deref trait.
 */
export class MutableBorrow<T extends MutableInstance> {
  readonly inner: T;
  private readonly _entityRef: Entity;

  constructor(entity: Entity, mutableFactory: new (entity: Entity) => T) {
    this._entityRef = entity;
    this.inner = new mutableFactory(entity);
  }

  /**
   * Extract the core mutable (for usage where the borrow wrapper is not needed).
   *
   * Rust: `pub fn into_core(self) -> T { self.mutable }`
   */
  intoCore(): T {
    return this.inner;
  }
}
