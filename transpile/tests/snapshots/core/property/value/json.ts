// MIRRORS: ankurah/core/src/property/value/json.rs
import { Struct, Result, JsonError, OwnershipFatal } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { Property } from '../index';
import { PropertyError } from '../traits';
import { Value } from '../../value/index';

export class Json extends Struct implements Property {
  readonly _0: unknown;

  constructor(_0: unknown) {
    super();
    this._0 = _0;
  }

  static new(value: unknown): Json {
    return new Json(value);
  }

  static null(): Json {
    return new Json(serdeJson.Value.Null);
  }

  static object(pairs: [string, unknown][]): Json {
    const map = [...pairs].map(([k, v]) => [k, v]);
    return new Json(serdeJson.Value.Object(map));
  }

  static array(items: unknown[]): Json {
    return new Json(serdeJson.Value.Array([...items]));
  }

  inner(): unknown {
    return this._0;
  }

  innerMut(): unknown {
    return this._0;
  }

  intoInner(): unknown {
    try {
      return this._0;
    } finally {
      this.drop();
    }
  }

  getPath(path: string[]): unknown | null {
    let current = this._0;
    for (const step of path) {
      const _r0 = ((current as Record<string, unknown>)?.[step] ?? null);
      if (_r0 == null) return null;
      current = _r0;
    }
    return current;
  }

  isNull(): boolean {
    return (this._0 === null);
  }

  isObject(): boolean {
    return (this._0 !== null && typeof this._0 === 'object' && !Array.isArray(this._0));
  }

  isArray(): boolean {
    return Array.isArray(this._0);
  }

  static default(): Json {
    return Json.null();
  }

  static fromValue(value: unknown): Json {
    return new Json(value);
  }

  deref(): unknown {
    return this._0;
  }

  derefMut(): unknown {
    return this._0;
  }

  intoValue(): Result<Value | null, PropertyError> {
    return Result.Ok(new Value('Json', { _0: structuredClone(this._0) }));
  }

  equals(other: Json): boolean {
    if (!this._0.equals(other._0)) return false;
    return true;
  }

  clone(): Json {
    return new Json(this._0.clone());
  }

  debug(): string {
    return `Json(${this._0})`;
  }

  encode(writer: BincodeWriter): void {
    this._0.encode(writer);
  }

  static decode(reader: BincodeReader): Json {
    const _0 = unknown.decode(reader);
    return new Json(_0);
  }

  toJSON(): unknown {
    return this._0;
  }

  static fromJson(value: unknown): Result<Json, JsonError> {
    try {
      const _r_0 = ((v: unknown) => Result.Ok(v))(value);
      if (_r_0.isErr()) return Result.Err(_r_0.unwrapErr());
      const _0 = _r_0.unwrap();
      return Result.Ok(new Json(_0));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export function Value_fromJson(json: Json): unknown {
  try {
    return json._0;
  } finally {
    json.drop();
  }
}

