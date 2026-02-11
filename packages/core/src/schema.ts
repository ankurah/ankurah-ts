// MIRRORS: ankurah/core/src/schema.rs

import type { PathExpr } from '@ankurah/ankql';
import type { PropertyError } from './property/traits.ts';
import type { ValueType } from './value/index.ts';

// ---------------------------------------------------------------------------
// CollectionSchema — trait for providing schema information about collections
// ---------------------------------------------------------------------------

/**
 * Interface for providing schema information about collections.
 *
 * Rust: `pub trait CollectionSchema`
 *
 * Provides field type lookups for a collection, used by predicate casting
 * and query compilation.
 */
export interface CollectionSchema {
  /**
   * Get the ValueType for a given field path.
   *
   * Rust: `fn field_type(&self, path: &PathExpr) -> Result<ValueType, PropertyError>`
   * Divergence: Throws PropertyError instead of returning Result [A8].
   */
  fieldType(path: PathExpr): ValueType;
}
