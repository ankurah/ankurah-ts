// MIRRORS: ankurah/proto/src/auth.rs

import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

/// Raw context data that can be transmitted between nodes - this may be a bearer token
/// or some other arbitrary data at the discretion of the Policy Agent
// Rust: fn serialize — SKIP: derived serde [E7]
// Rust: fn deserialize — SKIP: derived serde [E7]
export class AuthData extends Struct {
  readonly _0: Uint8Array;

  constructor(_0: Uint8Array = new Uint8Array(0)) {
    super();
    this._0 = _0;
  }

  static default(): AuthData {
    return new AuthData();
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this._0);
  }

  static decode(reader: BincodeReader): AuthData {
    return new AuthData(reader.readByteVec());
  }
}

export class Attestation extends Struct {
  readonly _0: Uint8Array;

  constructor(_0: Uint8Array = new Uint8Array(0)) {
    super();
    this._0 = _0;
  }

  static default(): Attestation {
    return new Attestation();
  }

  // Rust: derive(PartialEq)
  equals(other: Attestation): boolean {
    if (this._0.length !== other._0.length) return false;
    for (let i = 0; i < this._0.length; i++) {
      if (this._0[i] !== other._0[i]) return false;
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this._0);
  }

  static decode(reader: BincodeReader): Attestation {
    return new Attestation(reader.readByteVec());
  }
}

export class Attested<T> extends Struct {
  payload: T;
  attestations: AttestationSet;

  constructor(payload: T, attestations: AttestationSet = AttestationSet.default()) {
    super();
    this.payload = payload;
    this.attestations = attestations;
  }

  // Rust: fn opt
  static opt<T>(payload: T, attestation: Attestation | null): Attested<T> {
    const set = attestation ? new AttestationSet([attestation]) : AttestationSet.default();
    return new Attested(payload, set);
  }

  // Rust: fn fmt (Display for Attested<T>)
  toString(): string {
    return `Attested(${this.payload})`;
  }

  encode(writer: BincodeWriter, encodePayload: (w: BincodeWriter, p: T) => void): void {
    encodePayload(writer, this.payload);
    this.attestations.encode(writer);
  }

  static decode<T>(reader: BincodeReader, decodePayload: (r: BincodeReader) => T): Attested<T> {
    const payload = decodePayload(reader);
    const attestations = AttestationSet.decode(reader);
    return new Attested(payload, attestations);
  }
}

export class AttestationSet extends Struct {
  readonly _0: Attestation[];

  constructor(_0: Attestation[] = []) {
    super();
    this._0 = _0;
  }

  static default(): AttestationSet {
    return new AttestationSet();
  }

  // Rust: fn deref (Deref for AttestationSet)
  // impl Deref for AttestationSet — target: [Attestation]
  get length(): number {
    return this._0.length;
  }

  [Symbol.iterator](): Iterator<Attestation> {
    return this._0[Symbol.iterator]();
  }

  // Rust: fn push
  // impl AttestationSet
  push(attestation: Attestation): void {
    this._0.push(attestation);
  }

  equals(other: AttestationSet): boolean {
    if (this._0.length !== other._0.length) return false;
    for (let i = 0; i < this._0.length; i++) {
      if (!this._0[i].equals(other._0[i])) return false;
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    writer.writeVec(this._0, (w, a) => a.encode(w));
  }

  static decode(reader: BincodeReader): AttestationSet {
    return new AttestationSet(reader.readVec(r => Attestation.decode(r)));
  }
}

export class Principal extends Struct {
  constructor() {
    super();
  }

  // TODO — empty struct in Rust

  encode(writer: BincodeWriter): void {
    // Empty struct — no fields to encode
  }

  static decode(reader: BincodeReader): Principal {
    return new Principal();
  }
}
