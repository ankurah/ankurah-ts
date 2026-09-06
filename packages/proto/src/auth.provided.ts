// PROVIDED: Hand-written Attested<T> — generic type requires callback-based encode/decode.
// The transpiler never overwrites this file. Generated auth.ts re-exports this type.
//
// No toJSON here on purpose: Rust derives serde for Attested, and a derived struct with
// named fields writes `{"payload":…,"attestations":…}` — which is what JSON.stringify
// already produces from these two fields, in this declaration order. Whether that JSON
// matches Rust end to end depends on AttestationSet, a generated newtype that serde sees
// through and JSON.stringify does not; that belongs to the emitter, not to this file.

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

  // #[derive(Debug)] on a named-field struct — rustc prints
  // `Attested { payload: .., attestations: .. }`, each field through its own
  // Debug. `T` is whatever the instantiation put there, so the payload is
  // asked for its `debug()` and falls back to `String(..)` where it has none,
  // which is what a primitive payload wants.
  debug(): string {
    const payload = (this.payload as { debug?: () => string })?.debug?.() ?? String(this.payload);
    return `Attested { payload: ${payload}, attestations: ${this.attestations.debug()} }`;
  }

  equals(other: Attested<T>): boolean {
    return (this.payload as any).equals?.(other.payload) ?? this.payload === other.payload;
  }

  clone(): Attested<T> {
    const clonedPayload = (this.payload as any).clone?.() ?? this.payload;
    return new Attested(clonedPayload, this.attestations.clone());
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
