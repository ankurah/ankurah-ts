// MIRRORS: ankurah/core/src/property/value/entity_ref.rs
import { Struct, Result, unsupported } from '@ankurah/base';
import { EntityId, DecodeError } from '@ankurah/proto';
import { BincodeReader, BincodeWriter } from '../../codec';
import { Context } from '../../context';
import { RetrievalError } from '../../error';
import { Model, View } from '../../indexel';
import { Value } from '../../value/index';
import { Property } from '../index';
import { PropertyError } from '../traits';
import { Expr } from '@ankurah/ankql';

export class Ref<T extends Model> extends Struct implements Property {
  id: EntityId;

  constructor(id: EntityId) {
    super();
    this.id = id;
  }

  static new<T>(id: EntityId): Ref<T> {
    return new Ref(id, undefined /* PhantomData */);
  }

  static fromBase64<T>(s: string): Result<Ref<T>, DecodeError> {
    const _r0 = EntityId.fromBase64(s);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    return Result.Ok(new Ref(_r0.unwrap()));
  }

  id(): EntityId {
    return this.id.clone();
  }

  idRef(): EntityId {
    return this.id;
  }

  async get(ctx: Context): Promise<Result<View, RetrievalError>> {
    return await ctx.get(this.id.clone());
  }

  deref(): EntityId {
    return this.id;
  }

  asRef(): EntityId {
    return this.id;
  }

  borrow(): EntityId {
    return this.id;
  }

  static fromEntityId<T>(id: EntityId): Ref<T> {
    return new Ref(id);
  }

  static fromRefEntityId<T>(id: EntityId): Ref<T> {
    return new Ref(id.clone());
  }

  static tryFrom<T>(s: string): Result<Ref<T>, DecodeError> {
    return Ref.fromBase64(s);
  }

  toString(): string {
    return `${this.id.toBase64()}`;
  }

  static fromV<T, V>(view: V): Ref<Model> {
    return new Ref(view.id());
  }

  intoValue(): Result<Value | null, PropertyError> {
    return Result.Ok(new Value('EntityId', { _0: this.id.clone() }));
  }

  static fromValue<T>(value: Value | null): Result<Ref<T>, PropertyError> {
    unsupported('an arm of this consuming `Option` match tests inside the payload, and the port cannot both take a name out of that payload and release what is left of it here');
  }

  equals(other: Ref<T>): boolean {
    if (!this.id.equals(other.id)) return false;
    if (!this._phantom.equals(other._phantom)) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [this.id.hash(), this._phantom.hash()].map((p) => p.length + ':' + p).join('');
  }

  clone(): Ref<T> {
    return new Ref(this.id.clone(), this._phantom.clone());
  }

  debug(): string {
    return `Ref { id: ${this.id}, _phantom: ${this._phantom} }`;
  }

  encode(writer: BincodeWriter): void {
    this.id.encode(writer);
    this._phantom.encode(writer);
  }

  static decode(reader: BincodeReader): Ref<T> {
    const id = EntityId.decode(reader);
    const _phantom = PhantomData.decode(reader);
    return new Ref(id, _phantom);
  }
}

export function EntityId_fromRefT<T>(r: Ref<T>): EntityId {
  return r.id;
}

export function EntityId_fromRefRefT<T>(r: Ref<T>): EntityId {
  return r.id.clone();
}

export function Expr_fromRefT<T>(r: Ref<T>): Expr {
  return Expr.fromEntityId(r.id);
}

export function Expr_fromRefRefT<T>(r: Ref<T>): Expr {
  return Expr.fromEntityId((r.id));
}

