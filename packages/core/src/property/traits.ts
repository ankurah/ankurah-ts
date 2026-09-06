// MIRRORS: ankurah/core/src/property/traits.rs
import { Enum, Result } from '@ankurah/base';
import { RetrievalError } from '../error';
import { CastError } from '../value/cast';
import { Value } from '../value/index';

export type PropertyErrorV = {
  Missing: {};
  SerializeError: { _0: Error };
  DeserializeError: { _0: Error };
  RetrievalError: { _0: RetrievalError };
  InvalidVariant: { given: Value; ty: string };
  InvalidValue: { value: string; ty: string };
  TransactionClosed: {};
  CastError: { _0: CastError };
};

export class PropertyError extends Enum<PropertyErrorV> {

  equals(other: PropertyError): boolean {
    return this.toString() === other.toString();
  }

  static fromRetrievalError(retrieval: RetrievalError): PropertyError {
    return new PropertyError('RetrievalError', { _0: retrieval });
  }

  static fromError(e: Error): PropertyError {
    return new PropertyError('SerializeError', { _0: e });
  }

  debug(): string {
    return this.match({
      Missing: () => 'Missing',
      SerializeError: (v) => `SerializeError(${v._0})`,
      DeserializeError: (v) => `DeserializeError(${v._0})`,
      RetrievalError: (v) => `RetrievalError(${v._0.debug()})`,
      InvalidVariant: (v) => `InvalidVariant { given: ${v.given.debug()}, ty: ${JSON.stringify(v.ty)} }`,
      InvalidValue: (v) => `InvalidValue { value: ${JSON.stringify(v.value)}, ty: ${JSON.stringify(v.ty)} }`,
      TransactionClosed: () => 'TransactionClosed',
      CastError: (v) => `CastError(${v._0.debug()})`,
    });
  }

  override toString(): string {
    return this.match({
      Missing: () => 'property is missing',
      SerializeError: (v) => `serialization error: ${v._0}`,
      DeserializeError: (v) => `deserialization error: ${v._0}`,
      RetrievalError: (v) => `retrieval error: ${v._0}`,
      InvalidVariant: (v) => `invalid variant \`${v.given}\` for \`${v.ty}\``,
      InvalidValue: (v) => `invalid value \`${v.value}\` for \`${v.ty}\``,
      TransactionClosed: () => 'transaction is no longer alive',
      CastError: (v) => `cast error: ${v._0}`,
    });
  }
}

export interface InitializeWith<T> {
  initializeWith(entity: Entity, propertyName: PropertyName, value: T): Self;
}

export interface FromEntity {
  fromEntity(propertyName: PropertyName, entity: Entity): Self;
}

export interface FromActiveType<A> {
  fromActive(active: A): Result<Self, PropertyError>;
}

export function Error_fromPropertyError(arg: PropertyError): Error {
  try {
    return Error;
  } finally {
    arg.drop();
  }
}

