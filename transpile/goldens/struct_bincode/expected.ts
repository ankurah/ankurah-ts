// MIRRORS: ankurah/struct_bincode/src/input.rs
import { Struct, Result, JsonError, OwnershipFatal, UnsupportedShape } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

export class Envelope extends Struct {
  readonly id: bigint;
  readonly label: string;
  readonly payload: Uint8Array;

  constructor(id: bigint, label: string, payload: Uint8Array) {
    super();
    this.id = id;
    this.label = label;
    this.payload = payload;
  }

  static new(id: bigint, label: string, payload: Uint8Array): Envelope {
    return new Envelope(id, label, payload);
  }

  equals(other: Envelope): boolean {
    if (this.id !== other.id) return false;
    if (this.label !== other.label) return false;
    { if (this.payload.length !== other.payload.length) return false; for (let i = 0; i < this.payload.length; i++) { if (this.payload[i] !== other.payload[i]) return false; } }
    return true;
  }

  clone(): Envelope {
    return new Envelope(this.id, this.label, new Uint8Array(this.payload));
  }

  debug(): string {
    return `Envelope { id: ${String(this.id)}, label: ${JSON.stringify(this.label)}, payload: ${`[${Array.from(this.payload).map((e) => String(e)).join(', ')}]`} }`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeU64(this.id);
    writer.writeString(this.label);
    writer.writeByteVec(this.payload);
  }

  static decode(reader: BincodeReader): Envelope {
    const id = reader.readU64();
    const label = reader.readString();
    const payload = reader.readByteVec();
    return new Envelope(id, label, payload);
  }

  toJSON(): unknown {
    return { 'id': this.id, 'label': this.label, 'payload': Array.from(this.payload) };
  }

  static fromJson(value: unknown): Result<Envelope, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `Envelope`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('id' in _o)) {
        return Result.Err(JsonError.custom('missing field `id`'));
      }
      const _rid = ((v: unknown) => ((typeof v === 'bigint' && v >= 0n && v <= 18446744073709551615n) || (typeof v === 'number' && Number.isSafeInteger(v) && v >= 0 && v <= 9007199254740991) ? Result.Ok(BigInt(v as bigint | number)) : Result.Err(JsonError.custom('expected a u64'))))(_o['id']);
      if (_rid.isErr()) return Result.Err(_rid.unwrapErr());
      const id = _rid.unwrap();
      if (!('label' in _o)) {
        return Result.Err(JsonError.custom('missing field `label`'));
      }
      const _rlabel = ((v: unknown) => (typeof v === 'string' ? Result.Ok(v as string) : Result.Err(JsonError.custom('expected a string'))))(_o['label']);
      if (_rlabel.isErr()) return Result.Err(_rlabel.unwrapErr());
      const label = _rlabel.unwrap();
      if (!('payload' in _o)) {
        return Result.Err(JsonError.custom('missing field `payload`'));
      }
      const _rpayload = ((v: unknown) => (Array.isArray(v) && v.every((b) => typeof b === 'number' && Number.isInteger(b) && b >= 0 && b <= 255) ? Result.Ok(new Uint8Array(v as number[])) : Result.Err(JsonError.custom('expected an array of bytes'))))(_o['payload']);
      if (_rpayload.isErr()) return Result.Err(_rpayload.unwrapErr());
      const payload = _rpayload.unwrap();
      return Result.Ok(new Envelope(id, label, payload));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export class Signature extends Struct {
  _0: Uint8Array;

  constructor(_0: Uint8Array) {
    super();
    this._0 = _0;
  }

  equals(other: Signature): boolean {
    { if (this._0.length !== other._0.length) return false; for (let i = 0; i < this._0.length; i++) { if (this._0[i] !== other._0[i]) return false; } }
    return true;
  }

  clone(): Signature {
    return new Signature(new Uint8Array(this._0));
  }

  debug(): string {
    return `Signature(${`[${Array.from(this._0).map((e) => String(e)).join(', ')}]`})`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this._0);
  }

  static decode(reader: BincodeReader): Signature {
    const _0 = reader.readByteVec();
    return new Signature(_0);
  }

  toJSON(): unknown {
    return Array.from(this._0);
  }

  static fromJson(value: unknown): Result<Signature, JsonError> {
    try {
      const _r_0 = ((v: unknown) => (Array.isArray(v) && v.every((b) => typeof b === 'number' && Number.isInteger(b) && b >= 0 && b <= 255) ? Result.Ok(new Uint8Array(v as number[])) : Result.Err(JsonError.custom('expected an array of bytes'))))(value);
      if (_r_0.isErr()) return Result.Err(_r_0.unwrapErr());
      const _0 = _r_0.unwrap();
      return Result.Ok(new Signature(_0));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

