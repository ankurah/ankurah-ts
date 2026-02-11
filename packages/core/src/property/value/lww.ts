// MIRRORS: ankurah/core/src/property/value/lww.rs

import type { BroadcastId, Listener, Signal } from '@ankurah/signals';
import { ListenerGuard } from '@ankurah/signals';

import type { LWWBackend } from '../backend/lww.ts';
import type { PropertyName, PropertyFromValue, PropertyIntoValue } from '../index.ts';
import { PropertyError } from '../traits.ts';
import type { Value } from '../../value/index.ts';

import type { Entity } from '../../entity.ts';

// ---------------------------------------------------------------------------
// LWW<T> — active type wrapper for LWW property values
// ---------------------------------------------------------------------------

/**
 * Active type wrapper for Last-Writer-Wins property values.
 *
 * Provides typed get/set access to a single property managed by an LWWBackend.
 * The type parameter T represents the projected (user-facing) type.
 *
 * Rust: `pub struct LWW<T: Property>`
 * Divergence: Rust uses PhantomData<T> for the type parameter; TS uses conversion functions [E4].
 * Divergence: Rust uses Arc<LWWBackend>; TS uses plain reference [E8].
 */
export class LWW<T> implements Signal {
  readonly propertyName: PropertyName;
  readonly backend: LWWBackend;
  readonly entity: Entity;

  /**
   * Conversion function: T -> Value | null.
   * Mirrors Rust Property::into_value().
   */
  private readonly intoValue: PropertyIntoValue<T>;

  /**
   * Conversion function: Value | null -> T.
   * Mirrors Rust Property::from_value().
   */
  private readonly fromValue: PropertyFromValue<T>;

  constructor(
    propertyName: PropertyName,
    backend: LWWBackend,
    entity: Entity,
    intoValue: PropertyIntoValue<T>,
    fromValue: PropertyFromValue<T>,
  ) {
    this.propertyName = propertyName;
    this.backend = backend;
    this.entity = entity;
    this.intoValue = intoValue;
    this.fromValue = fromValue;
  }

  // ── Typed access ──

  /**
   * Set the property value.
   * Throws PropertyError if the entity is not writable (transaction closed).
   *
   * Rust: `pub fn set(&self, value: &T) -> Result<(), PropertyError>`
   */
  set(value: T): void {
    if (this.entity && typeof this.entity.isWritable === 'function' && !this.entity.isWritable()) {
      throw PropertyError.transactionClosed();
    }
    const converted = this.intoValue(value);
    this.backend.set(this.propertyName, converted);
  }

  /**
   * Get the property value as the projected type T.
   * Throws PropertyError on conversion failure.
   *
   * Rust: `pub fn get(&self) -> Result<T, PropertyError>`
   */
  get(): T {
    const value = this.getValue();
    return this.fromValue(value);
  }

  /**
   * Get the raw Value from the backend.
   *
   * Rust: `pub fn get_value(&self) -> Option<Value>`
   */
  getValue(): Value | null {
    return this.backend.get(this.propertyName);
  }

  // ── Signal interface ──

  /**
   * Listen to changes for this property.
   *
   * Rust: `impl<T: Property> Signal for LWW<T>`
   */
  listen(listener: Listener): ListenerGuard {
    return this.backend.listenField(this.propertyName, listener);
  }

  /**
   * Get the broadcast identifier for this property.
   *
   * Rust: `fn broadcast_id(&self) -> BroadcastId`
   */
  broadcastId(): BroadcastId {
    return this.backend.fieldBroadcastId(this.propertyName);
  }

  // ── Debug ──

  toString(): string {
    return `LWW { propertyName: ${this.propertyName} }`;
  }
}
