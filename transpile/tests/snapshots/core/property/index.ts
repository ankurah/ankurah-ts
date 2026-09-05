// MIRRORS: ankurah/core/src/property/mod.rs
import { Result, Ref } from '@ankurah/base';
import { EntityId } from '@ankurah/proto';
import { PropertyError } from './traits';
import { Value } from '../value/index';
export * from './backend';
export * from './traits';
export * from './value';

export interface Property {
  intoValue(): Result<Value | null, PropertyError>;
  fromValue(value: Value | null): Result<Self, PropertyError>;
}

export type PropertyName = string;

export function Option_intoValue<T extends Property>(self: T | null): Result<Value | null, PropertyError> {
  const _r0 = Property.intoValue(value);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  if (self != null) {
    const value = self;
    return Result.Ok(_r0.unwrap());
  } else {
    return Result.Ok(null);
  }
}

export function Option_fromValue<T extends Property>(value: Value | null): Result<T | null, PropertyError> {
  const _v = T.fromValue(value);
  if (_v.isOk()) {
    const value = _v.unwrap();
    return Result.Ok(value);
  } else {
    const err = _v.unwrapErr();
    return Result.Err(err);
  }
}

export function Cow_Str_intoValue(self: Cow<string>): Result<Value | null, PropertyError> {
  return Result.Ok(new Value('String', { _0: self.toString() }));
}

export function Cow_Str_fromValue(value: Value | null): Result<Cow<string>, PropertyError> {
  const _v = value;
  if (_v != null && (_v.is('String'))) {
    const { _0: value } = _v.value;
    return Result.Ok(value);
  } else if (_v != null) {
    const variant = _v;
    return Result.Err(new PropertyError('InvalidVariant', { given: variant, ty: '$ ty'.clone() }));
  } else {
    return Result.Err(new PropertyError('Missing', {}));
  }
}

export function Value_from(value: string): Value {
  return new Value('String', { _0: value });
}

export function Property_dispatch_intoValue(self: unknown): Result<Value | null, PropertyError> {
  if (self instanceof Option) return Option_intoValue(self as any);
  if (self instanceof Cow) return Cow_Str_intoValue(self as any);
  if (self instanceof Ref) return (self as any).intoValue();
  if (self instanceof Json) return (self as any).intoValue();
  if (self instanceof Item) return Item_intoValue(self as any);
  throw new Error(`BUG: no Property impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

