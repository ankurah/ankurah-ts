// MIRRORS: ankurah/core/src/property/value/yrs.rs
// Exception E5: yrs.rs -> yrs_string.ts (Yrs library rename; filename preserves Rust type name)

import type { BroadcastId, Listener, Signal } from '@ankurah/signals';
import { ListenerGuard } from '@ankurah/signals';

import { YjsBackend } from '../backend/yjs.ts';
import type { PropertyName } from '../index.ts';
import { PropertyError } from '../traits.ts';
import { MutationError } from '../../error.ts';

import type { Entity } from '../../entity.ts';

// ---------------------------------------------------------------------------
// YrsString<T>
// ---------------------------------------------------------------------------

/**
 * Active type wrapper for CRDT text properties backed by Yjs.
 *
 * Rust: `pub struct YrsString<Projected> { property_name, backend, entity, phantom }`
 * Divergence: No PhantomData needed — TS generics are erased at runtime [A6].
 * Divergence: No Arc<YrsBackend> — plain reference [E8].
 *
 * Starting with basic string type operations.
 */
export class YrsString<Projected = string> implements Signal {
  readonly propertyName: PropertyName;
  readonly backend: YjsBackend;
  readonly entity: Entity;

  constructor(propertyName: PropertyName, backend: YjsBackend, entity: Entity) {
    this.propertyName = propertyName;
    this.backend = backend;
    this.entity = entity;
  }

  // ── Value access ────────────────────────────────────────────────────

  /**
   * Get the current string value, or null if no content has been inserted.
   *
   * Rust: `pub fn value(&self) -> Option<String>`
   */
  value(): string | null {
    return this.backend.getString(this.propertyName);
  }

  // ── Mutation methods ────────────────────────────────────────────────

  /**
   * Insert text at a given index.
   *
   * Rust: `pub fn insert(&self, index: u32, value: &str) -> Result<(), MutationError>`
   * Throws MutationError if the entity's transaction is no longer alive.
   */
  insert(index: number, value: string): void {
    if (this.entity && typeof this.entity.isWritable === 'function' && !this.entity.isWritable()) {
      throw MutationError.propertyError(PropertyError.transactionClosed());
    }
    this.backend.insert(this.propertyName, index, value);
  }

  /**
   * Delete a range of characters starting at the given index.
   *
   * Rust: `pub fn delete(&self, index: u32, length: u32) -> Result<(), MutationError>`
   * Throws MutationError if the entity's transaction is no longer alive.
   */
  delete(index: number, length: number): void {
    if (this.entity && typeof this.entity.isWritable === 'function' && !this.entity.isWritable()) {
      throw MutationError.propertyError(PropertyError.transactionClosed());
    }
    this.backend.delete(this.propertyName, index, length);
  }

  /**
   * Overwrite a range: delete `length` chars starting at `start`, then insert `text` at `start`.
   *
   * Rust: `pub fn overwrite(&self, start: u32, length: u32, value: &str) -> Result<(), MutationError>`
   * Throws MutationError if the entity's transaction is no longer alive.
   */
  overwrite(start: number, length: number, text: string): void {
    if (this.entity && typeof this.entity.isWritable === 'function' && !this.entity.isWritable()) {
      throw MutationError.propertyError(PropertyError.transactionClosed());
    }
    this.backend.delete(this.propertyName, start, length);
    this.backend.insert(this.propertyName, start, text);
  }

  /**
   * Replace the entire text content.
   *
   * Rust: `pub fn replace(&self, value: &str) -> Result<(), MutationError>`
   * Throws MutationError if the entity's transaction is no longer alive.
   */
  replace(text: string): void {
    if (this.entity && typeof this.entity.isWritable === 'function' && !this.entity.isWritable()) {
      throw MutationError.propertyError(PropertyError.transactionClosed());
    }
    const current = this.value() ?? '';
    this.backend.delete(this.propertyName, 0, current.length);
    this.backend.insert(this.propertyName, 0, text);
  }

  // ── Signal interface ────────────────────────────────────────────────

  /**
   * Listen to changes to this text field.
   *
   * Rust: `impl Signal for YrsString<P> { fn listen(&self, listener: Listener) -> ListenerGuard }`
   */
  listen(listener: Listener): ListenerGuard {
    return this.backend.listenField(this.propertyName, listener);
  }

  /**
   * Get the broadcast identifier for this text field.
   *
   * Rust: `fn broadcast_id(&self) -> BroadcastId`
   */
  broadcastId(): BroadcastId {
    return this.backend.fieldBroadcastId(this.propertyName);
  }

  // ── Static factory methods (trait impls in Rust) ────────────────────

  /**
   * Construct from an Entity and property name.
   *
   * Rust: `impl FromEntity for YrsString<P>`
   */
  static fromEntity<P = string>(propertyName: PropertyName, entity: Entity): YrsString<P> {
    const backend: YjsBackend = entity.getBackend(YjsBackend);
    return new YrsString<P>(propertyName, backend, entity);
  }

  /**
   * Construct from an Entity with an initial string value.
   *
   * Rust: `impl InitializeWith<String> for YrsString<P>`
   */
  static initializeWith<P = string>(
    entity: Entity,
    propertyName: PropertyName,
    value: string,
  ): YrsString<P> {
    const instance = YrsString.fromEntity<P>(propertyName, entity);
    instance.insert(0, value);
    return instance;
  }

  /**
   * Construct from an Entity with an optional initial string value.
   *
   * Rust: `impl InitializeWith<Option<String>> for YrsString<P>`
   */
  static initializeWithOptional<P = string>(
    entity: Entity,
    propertyName: PropertyName,
    value: string | null,
  ): YrsString<P> {
    const instance = YrsString.fromEntity<P>(propertyName, entity);
    if (value !== null) {
      instance.insert(0, value);
    }
    return instance;
  }
}

// ---------------------------------------------------------------------------
// FromActiveType implementations (standalone functions in TS)
// ---------------------------------------------------------------------------

/**
 * Extract a string from a YrsString active type.
 * Throws PropertyError.Missing if no value has been inserted.
 *
 * Rust: `impl FromActiveType<YrsString<P>> for String`
 */
export function stringFromYrsString<P>(active: YrsString<P>): string {
  const val = active.value();
  if (val === null) {
    throw PropertyError.missing();
  }
  return val;
}

/**
 * Extract an optional string from a YrsString active type.
 * Returns null if no value has been inserted.
 *
 * Rust: `impl<S: FromActiveType<YrsString<P>>> FromActiveType<YrsString<P>> for Option<S>`
 */
export function optionalStringFromYrsString<P>(active: YrsString<P>): string | null {
  return active.value();
}
