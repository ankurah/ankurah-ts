// MIRRORS: ankurah/proto/src/auth.rs
import { Struct, Result, JsonError } from '@ankurah/base';
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

  debug(): string {
    return `AuthData(${`[${Array.from(this._0).map((e) => String(e)).join(', ')}]`})`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this._0);
  }

  static decode(reader: BincodeReader): AuthData {
    const _0 = reader.readByteVec();
    return new AuthData(_0);
  }

  toJSON(): unknown {
    return Array.from(this._0);
  }

  static fromJson(value: unknown): Result<AuthData, JsonError> {
    try {
      return Result.Ok(new AuthData(((v: unknown) => new Uint8Array(v as number[]))(value)));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
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

  debug(): string {
    return `Attestation(${`[${Array.from(this._0).map((e) => String(e)).join(', ')}]`})`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this._0);
  }

  static decode(reader: BincodeReader): Attestation {
    const _0 = reader.readByteVec();
    return new Attestation(_0);
  }

  toJSON(): unknown {
    return Array.from(this._0);
  }

  static fromJson(value: unknown): Result<Attestation, JsonError> {
    try {
      return Result.Ok(new Attestation(((v: unknown) => new Uint8Array(v as number[]))(value)));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
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

  deref(): Attestation[] {
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

  debug(): string {
    return `AttestationSet(${`[${Array.from(this._0).map((e) => e.debug()).join(', ')}]`})`;
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

  toJSON(): unknown {
    return this._0;
  }

  static fromJson(value: unknown): Result<AttestationSet, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      return Result.Ok(new AttestationSet(((v: unknown) => (v as unknown[]).map((v) => _take(Attestation.fromJson(v))))(value)));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export class Principal extends Struct {

  clone(): Principal {
    return new Principal();
  }

  debug(): string {
    return 'Principal';
  }

  encode(writer: BincodeWriter): void {
  }

  static decode(reader: BincodeReader): Principal {
    return new Principal();
  }

  toJSON(): unknown {
    return null;
  }

  static fromJson(value: unknown): Result<Principal, JsonError> {
    try {
      return Result.Ok(new Principal());
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
  }
}

