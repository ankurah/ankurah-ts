// MIRRORS: ankurah/core/src/property/mod.rs
import { Result, Ref, dropOwned, unsupported } from '@ankurah/base';
import { Value } from '../value/index';
import { PropertyError } from './traits';
export * from './backend';
export * from './traits';
export * from './value';

export interface Property {
  intoValue(): Result<Value | null, PropertyError>;
  fromValue(value: Value | null): Result<Self, PropertyError>;
}

export type PropertyName = string;

export function Option_intoValue<T extends Property>(self: T | null): Result<Value | null, PropertyError> {
  if (self != null) {
    const value = self;
    const _r0 = Property.intoValue(value);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
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
    const _v1 = _v.unwrapErr();
    if (_v1.is('Missing')) {
      const _v2 = _v1;
      try {
        return Result.Ok(null);
      } finally {
        dropOwned(_v2);
      }
    }
    {
      const err = _v1;
      return Result.Err(err);
    }
  }
}

export function Cow_Str_intoValue(self: Cow<string>): Result<Value | null, PropertyError> {
  return Result.Ok(new Value('String', { _0: self }));
}

export function Cow_Str_fromValue(value: Value | null): Result<Cow<string>, PropertyError> {
  unsupported('an arm of this consuming `Option` match tests inside the payload, and the port cannot both take a name out of that payload and release what is left of it here');
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

