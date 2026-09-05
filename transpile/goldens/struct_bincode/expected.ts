// MIRRORS: ankurah/struct_bincode/src/input.rs
import { Struct, Result, JsonError } from '@ankurah/base';
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
    return {
      'id': Number(this.id),
      'label': this.label,
      'payload': Array.from(this.payload),
    };
  }

  static fromJson(value: unknown): Result<Envelope, JsonError> {
    try {
      const o = value as Record<string, unknown>;
      const id = ((v: unknown) => BigInt(v as number))(o['id']);
      const label = ((v: unknown) => v as string)(o['label']);
      const payload = ((v: unknown) => new Uint8Array(v as number[]))(o['payload']);
      return Result.Ok(new Envelope(id, label, payload));
    } catch (e) {
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
      return Result.Ok(new Signature(((v: unknown) => new Uint8Array(v as number[]))(value)));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
  }
}

