// MIRRORS: ankurah/storage/indexeddb-wasm/src/util/object.rs
import { Struct, Result, AnyhowError } from '@ankurah/base';
import { MutationError, RetrievalError } from '@ankurah/core';

export class Object extends Struct {
  obj: SendWrapper<unknown>;

  constructor(obj: SendWrapper<unknown>) {
    super();
    this.obj = obj;
  }

  static new(obj: unknown): Object {
    return new Object(SendWrapper.new(obj));
  }

  get<T extends TryFrom>(key: unknown): Result<T, RetrievalError> {
    const _r0 = jsSys.Reflect.get(this.obj, key).mapErr((_e) => new RetrievalError('StorageError', { _0: AnyhowError.msg(`Failed to get ${(typeof key === 'string' ? key : null).unwrapOrDefault()}`) }));
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const v = _r0.unwrap();
    return v.tryInto().mapErr((_e) => new RetrievalError('StorageError', { _0: AnyhowError.msg(`Failed to convert ${(typeof key === 'string' ? key : null).unwrapOrDefault()}`) }));
  }

  getOpt<T extends TryFrom>(key: unknown): Result<T | null, RetrievalError> {
    const _r0 = jsSys.Reflect.get(this.obj, key).mapErr((_e) => new RetrievalError('StorageError', { _0: AnyhowError.msg(`Failed to get ${(typeof key === 'string' ? key : null).unwrapOrDefault()}`) }));
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const v = _r0.unwrap();
    if ((v === null) || (v === undefined)) {
      return Result.Ok(null);
    }
    const _r1 = v.tryInto().mapErr((_e) => {
      return new RetrievalError('StorageError', { _0: AnyhowError.msg(`Failed to convert ${(typeof key === 'string' ? key : null).unwrapOrDefault()}`) });
    });
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    return Result.Ok(_r1.unwrap());
  }

  set<K, V>(key: K, value: V): Result<boolean, MutationError> {
    const jsKey = key;
    const _r0 = value.tryInto().mapErr((e) => new MutationError('General', { _0: e }));
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const jsValue = _r0.unwrap();
    return jsSys.Reflect.set(this.obj, jsKey, jsValue).mapErr((_e) => new MutationError('FailedToSetProperty', { _0: 'field', _1: (typeof jsValue === 'string' ? jsValue : null).unwrapOrDefault() }));
  }

  deref(): unknown {
    return this.obj;
  }
}

export class Property extends Struct {
  key: SendWrapper<unknown>;
  name: string;

  constructor(key: SendWrapper<unknown>, name: string) {
    super();
    this.key = key;
    this.name = name;
  }

  // A `&T` field is a borrow: dropping this releases the borrow and nothing
  // else, so the cascade must not walk it.
  protected override ownedFields(): unknown[] {
    return [this.key];
  }

  static new(key: string): Property {
    return new Property(SendWrapper.new(key), key);
  }

  deref(): unknown {
    return this.key;
  }

  toString(): string {
    return `${this.name}`;
  }
}

export function JsValue_fromProperty(prop: Property): unknown {
  return (prop.key);
}

