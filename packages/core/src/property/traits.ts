// MIRRORS: ankurah/core/src/property/traits.rs

import type { Entity } from '../entity.ts';
import type { Value } from '../value/index.ts';
import type { CastErrorException } from '../value/cast.ts';
import { RetrievalError } from '../error.ts';

// Use CastErrorException (Error subclass) in PropertyError, not the CastError union type
type CastError = CastErrorException;

import type { PropertyName } from './index.ts';

// ---------------------------------------------------------------------------
// PropertyError
// ---------------------------------------------------------------------------

/**
 * Error enum for property operations.
 *
 * Rust: `enum PropertyError { Missing, SerializeError, DeserializeError, ... }`
 * TS: Error subclass with a `kind` discriminant [A8].
 */
export type PropertyErrorKind =
  | 'Missing'
  | 'SerializeError'
  | 'DeserializeError'
  | 'RetrievalError'
  | 'InvalidVariant'
  | 'InvalidValue'
  | 'TransactionClosed'
  | 'CastError';

export class PropertyError extends Error {
  readonly kind: PropertyErrorKind;
  readonly detail?: unknown;

  constructor(kind: PropertyErrorKind, message: string, detail?: unknown) {
    super(message);
    this.name = 'PropertyError';
    this.kind = kind;
    this.detail = detail;
  }

  /** Check if this is a Missing error */
  isMissing(): boolean {
    return this.kind === 'Missing';
  }

  /** String equality comparison, matching Rust PartialEq impl */
  equals(other: PropertyError): boolean {
    return this.message === other.message;
  }

  // ── Static factory helpers ──

  static missing(): PropertyError {
    return new PropertyError('Missing', 'property is missing');
  }

  static serializeError(err: Error): PropertyError {
    return new PropertyError('SerializeError', `serialization error: ${err.message}`, err);
  }

  static deserializeError(err: Error): PropertyError {
    return new PropertyError('DeserializeError', `deserialization error: ${err.message}`, err);
  }

  static retrievalError(err: RetrievalError): PropertyError {
    return new PropertyError('RetrievalError', `retrieval error: ${err.message}`, err);
  }

  static invalidVariant(given: Value, ty: string): PropertyError {
    return new PropertyError('InvalidVariant', `invalid variant \`${String(given)}\` for \`${ty}\``, { given, ty });
  }

  static invalidValue(value: string, ty: string): PropertyError {
    return new PropertyError('InvalidValue', `invalid value \`${value}\` for \`${ty}\``, { value, ty });
  }

  static transactionClosed(): PropertyError {
    return new PropertyError('TransactionClosed', 'transaction is no longer alive');
  }

  static castError(err: CastError): PropertyError {
    return new PropertyError('CastError', `cast error: ${err.message}`, err);
  }

  /** Convert from RetrievalError, matching Rust From<RetrievalError> for PropertyError */
  static fromRetrievalError(err: RetrievalError): PropertyError {
    return PropertyError.retrievalError(err);
  }
}

// ---------------------------------------------------------------------------
// InitializeWith<T>
// ---------------------------------------------------------------------------

/**
 * Trait for types that can be initialized with a value on an entity.
 *
 * Rust: `trait InitializeWith<T> { fn initialize_with(entity: &Entity, property_name: PropertyName, value: &T) -> Self; }`
 */
export interface InitializeWith<T> {
  // Note: In Rust this is a static method on the trait. In TS, interfaces cannot have
  // static methods. Implementors should provide a static `initializeWith` factory or
  // a constructor that accepts these arguments.
}

/**
 * Factory function type for InitializeWith. Implementors provide a function matching
 * this signature rather than implementing an interface with a static method.
 */
export type InitializeWithFactory<T, R> = (
  entity: Entity,
  propertyName: PropertyName,
  value: T,
) => R;

// ---------------------------------------------------------------------------
// FromEntity
// ---------------------------------------------------------------------------

/**
 * Trait for types that can be constructed from an Entity and a property name.
 *
 * Rust: `trait FromEntity { fn from_entity(property_name: PropertyName, entity: &Entity) -> Self; }`
 */
export interface FromEntity {
  // Note: In Rust this is a static method on the trait. In TS, interfaces cannot have
  // static methods. Implementors should provide a static `fromEntity` factory.
}

/**
 * Factory function type for FromEntity.
 */
export type FromEntityFactory<R> = (
  propertyName: PropertyName,
  entity: Entity,
) => R;

// ---------------------------------------------------------------------------
// FromActiveType<A>
// ---------------------------------------------------------------------------

/**
 * Trait for types that can be projected from an active (mutable) type.
 *
 * Rust: `trait FromActiveType<A> { fn from_active(active: A) -> Result<Self, PropertyError>; }`
 * TS: Throws PropertyError on failure [A8].
 */
export interface FromActiveType<A> {
  // Note: In Rust this is a static method on the trait. In TS, interfaces cannot have
  // static methods. Implementors should provide a static `fromActive` factory.
}

/**
 * Factory function type for FromActiveType.
 * Throws PropertyError on failure.
 */
export type FromActiveTypeFactory<A, R> = (active: A) => R;
