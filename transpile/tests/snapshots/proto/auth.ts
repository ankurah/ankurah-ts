// MIRRORS: ankurah/proto/src/auth.rs
import { Struct } from '@ankurah/base';
import { Attested } from './auth.provided';
import { BincodeReader, BincodeWriter } from './codec';
export { Attested };

export class AuthData extends Struct {
  readonly _0: Uint8Array;

  constructor(_0: Uint8Array) {
    super();
    this._0 = _0;
  }

  clone(): AuthData {
    return new AuthData(new Uint8Array(this._0));
  }

  static default(): AuthData {
    return new AuthData(new Uint8Array(0));
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this._0);
  }

  static decode(reader: BincodeReader): AuthData {
    const _0 = reader.readByteVec();
    return new AuthData(_0);
  }
}

export class Attestation extends Struct {
  readonly _0: Uint8Array;

  constructor(_0: Uint8Array) {
    super();
    this._0 = _0;
  }

  equals(other: Attestation): boolean {
    { if (this._0.length !== other._0.length) return false; for (let i = 0; i < this._0.length; i++) { if (this._0[i] !== other._0[i]) return false; } }
    return true;
  }

  clone(): Attestation {
    return new Attestation(new Uint8Array(this._0));
  }

  static default(): Attestation {
    return new Attestation(new Uint8Array(0));
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this._0);
  }

  static decode(reader: BincodeReader): Attestation {
    const _0 = reader.readByteVec();
    return new Attestation(_0);
  }
}

export class AttestationSet extends Struct {
  readonly _0: Attestation[];

  constructor(_0: Attestation[]) {
    super();
    this._0 = _0;
  }

  push(attestation: Attestation): void {
    this._0.push(attestation);
  }

  deref(): Target {
    return this._0;
  }

  equals(other: AttestationSet): boolean {
    { if (this._0.length !== other._0.length) return false; for (let i = 0; i < this._0.length; i++) { if (!this._0[i].equals(other._0[i])) return false; } }
    return true;
  }

  static default(): AttestationSet {
    return new AttestationSet([]);
  }

  clone(): AttestationSet {
    return new AttestationSet(this._0.map(e => e.clone()));
  }

  get length(): number {
    return this._0.length;
  }

  [Symbol.iterator](): Iterator<any> {
    return this._0[Symbol.iterator]();
  }

  encode(writer: BincodeWriter): void {
    writer.writeVec(this._0, (w, item) => item.encode(w));
  }

  static decode(reader: BincodeReader): AttestationSet {
    const _0 = reader.readVec((r) => Attestation.decode(r));
    return new AttestationSet(_0);
  }
}

export class Principal extends Struct {

  clone(): Principal {
    return new Principal();
  }

  encode(writer: BincodeWriter): void {
  }

  static decode(reader: BincodeReader): Principal {
    return new Principal();
  }
}

