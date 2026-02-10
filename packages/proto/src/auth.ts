// MIRRORS: ankurah/proto/src/auth.rs

import { BincodeReader, BincodeWriter } from './codec';

/**
 * AuthData: raw context data transmitted between nodes (e.g., bearer token).
 * Derived serde — serialized as Vec<u8> (u64 length + bytes).
 */
export class AuthData {
  readonly data: Uint8Array;

  constructor(data: Uint8Array = new Uint8Array(0)) {
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

/**
 * Attestation: opaque attestation bytes.
 * Derived serde — serialized as Vec<u8> (u64 length + bytes).
 */
export class Attestation {
  readonly data: Uint8Array;

  constructor(data: Uint8Array = new Uint8Array(0)) {
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

/**
 * AttestationSet: Vec<Attestation>.
 * Derived serde — serialized as Vec<Attestation>.
 */
export class AttestationSet {
  readonly attestations: Attestation[];

  constructor(attestations: Attestation[] = []) {
    this.attestations = attestations;
  }

  static default(): AttestationSet {
    return new AttestationSet();
  }

  get length(): number {
    return this.attestations.length;
  }

  push(attestation: Attestation): void {
    this.attestations.push(attestation);
  }

  [Symbol.iterator](): Iterator<Attestation> {
    return this.attestations[Symbol.iterator]();
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

/**
 * Attested<T>: wrapper pairing a payload with attestations.
 * Derived serde — serialized as struct { payload: T, attestations: AttestationSet }.
 */
export class Attested<T> {
  payload: T;
  attestations: AttestationSet;

  constructor(payload: T, attestations: AttestationSet = AttestationSet.default()) {
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

/**
 * Principal: placeholder type (TODO in Rust).
 * Derived serde — serialized as empty struct (no fields).
 */
export class Principal {
  encode(writer: BincodeWriter): void {
    // Empty struct — no fields to encode
  }

  static decode(reader: BincodeReader): Principal {
    return new Principal();
  }
}
