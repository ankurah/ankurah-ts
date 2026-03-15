// MIRRORS: ankurah/core/src/property/mod.rs

// ── Modules (matching Rust `pub mod` declarations) ──

export * from './backend/index.ts';
export * from './traits.ts';
export * from './value/lww.ts';
export * from './value/yrs_string.ts';

import type { Value } from '../value/index.ts';

// ---------------------------------------------------------------------------
// PropertyName
// ---------------------------------------------------------------------------

/**
 * Type alias for property names (field names within an entity).
 *
 * Rust: `pub type PropertyName = String;`
 */
export type PropertyName = string;

// ---------------------------------------------------------------------------
// Property interface
// ---------------------------------------------------------------------------

/**
 * Trait for types that can be serialized to/from a Value for storage in a PropertyBackend.
 *
 * Rust: `pub trait Property: Sized { fn into_value(&self) -> Result<Option<Value>, PropertyError>; fn from_value(value: Option<Value>) -> Result<Self, PropertyError>; }`
 * TS: Interface. Throws PropertyError on failure [A8].
 *
 * Note: The Rust crate provides blanket impls via a macro (`into!`) for primitive types
 * (String, i16, i32, i64, f64, bool, EntityId, Vec<u8>) and for Option<T: Property>.
 * Those impls will be ported when the Value module is ported, as standalone functions
 * or as part of the Value type itself.
 */
export interface Property<T = unknown> {
  /**
   * Serialize this value into a Value for storage.
   * Returns null for "no value" (e.g., Option::None).
   * Throws PropertyError on failure.
   *
   * Rust: `fn into_value(&self) -> Result<Option<Value>, PropertyError>`
   */
  intoValue(): Value | null;

  /**
   * Deserialize from a Value (or null for missing).
   * Throws PropertyError on failure.
   *
   * Rust: `fn from_value(value: Option<Value>) -> Result<Self, PropertyError>`
   * Note: This is a static method in Rust. In TS it is modeled as a standalone function
   * type since interfaces cannot have static methods.
   */
  // fromValue is a static factory in Rust; see PropertyFromValue below
}

/**
 * Factory function type for the static `from_value` method of the Property trait.
 * Throws PropertyError on failure.
 *
 * Rust: `fn from_value(value: Option<Value>) -> Result<Self, PropertyError>`
 */
export type PropertyFromValue<T> = (value: Value | null) => T;

/**
 * Factory function type for the `into_value` direction.
 * Throws PropertyError on failure.
 *
 * Rust: `fn into_value(&self) -> Result<Option<Value>, PropertyError>`
 */
export type PropertyIntoValue<T> = (value: T) => Value | null;
