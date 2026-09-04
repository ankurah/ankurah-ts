// MIRRORS: ankurah/struct_bincode/src/input.rs
import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

export class Envelope extends Struct {
  readonly id: bigint | number;
  readonly label: string;
  readonly payload: Uint8Array;

  constructor(id: bigint | number, label: string, payload: Uint8Array) {
    super();
    this.id = id;
    this.label = label;
    this.payload = payload;
  }

  static new(id: bigint | number, label: string, payload: Uint8Array): Envelope {
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

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this._0);
  }

  static decode(reader: BincodeReader): Signature {
    const _0 = reader.readByteVec();
    return new Signature(_0);
  }
}

