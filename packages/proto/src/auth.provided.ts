// PROVIDED: Hand-written Attested<T> — generic type requires callback-based encode/decode.
// The transpiler never overwrites this file. Generated auth.ts re-exports this type.

import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { Attestation, AttestationSet } from './auth';

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
