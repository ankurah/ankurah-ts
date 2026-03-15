// MIRRORS: ankurah/proto/src/auth.rs

import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

/// Raw context data that can be transmitted between nodes - this may be a bearer token
/// or some other arbitrary data at the discretion of the Policy Agent
export class AuthData extends Struct {
  readonly data: Uint8Array;

  constructor(data: Uint8Array = new Uint8Array(0)) {
    super();
    this.data = data;
  }

  static default(): AuthData {
    return new AuthData();
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this.data);
  }

  static decode(reader: BincodeReader): AuthData {
    return new AuthData(reader.readByteVec());
  }
}

export class Attestation extends Struct {
  readonly data: Uint8Array;

  constructor(data: Uint8Array = new Uint8Array(0)) {
    super();
    this.data = data;
  }

  static default(): Attestation {
    return new Attestation();
  }

  equals(other: Attestation): boolean {
    if (this.data.length !== other.data.length) return false;
    for (let i = 0; i < this.data.length; i++) {
      if (this.data[i] !== other.data[i]) return false;
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this.data);
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

  static opt<T>(payload: T, attestation: Attestation | null): Attested<T> {
    const set = attestation ? new AttestationSet([attestation]) : AttestationSet.default();
    return new Attested(payload, set);
  }

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
  readonly attestations: Attestation[];

  constructor(attestations: Attestation[] = []) {
    super();
    this.attestations = attestations;
  }

  static default(): AttestationSet {
    return new AttestationSet();
  }

  // impl Deref for AttestationSet — target: [Attestation]
  get length(): number {
    return this.attestations.length;
  }

  [Symbol.iterator](): Iterator<Attestation> {
    return this.attestations[Symbol.iterator]();
  }

  // impl AttestationSet
  push(attestation: Attestation): void {
    this.attestations.push(attestation);
  }

  equals(other: AttestationSet): boolean {
    if (this.attestations.length !== other.attestations.length) return false;
    for (let i = 0; i < this.attestations.length; i++) {
      if (!this.attestations[i].equals(other.attestations[i])) return false;
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    writer.writeVec(this.attestations, (w, a) => a.encode(w));
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
