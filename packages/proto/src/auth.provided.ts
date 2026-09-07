// PROVIDED: Hand-written Attested<T> — generic type requires callback-based encode/decode.
// The transpiler never overwrites this file. Generated auth.ts re-exports this type.
//
// The JSON halves take the payload's reader and writer, one callback each, the way
// `encode`/`decode` already take the payload's codec: `Attested<T>` cannot have one
// `fromJson` for every `T`, and Rust reads the payload through the `Deserialize` the
// instantiation supplies. Rust derives serde for `Attested`, and a derived struct with
// named fields writes `{"payload":…,"attestations":…}`; `AttestationSet` is a newtype,
// which serde sees through and writes as the array inside it, so its own `toJSON` is
// what says that rather than `JSON.stringify` of the object.

import { JsonError, OwnershipFatal, Result, Struct, UnsupportedShape, debugValue, dropOwned } from '@ankurah/base';
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
  // Debug. `T` is whatever the instantiation put there, so the payload goes
  // through the runtime's `debugValue`, which decides from the value's own
  // surface: a string is a Rust `String` and prints QUOTED, a number prints as
  // itself, `null` is `None`, a sequence prints element-wise, and an object
  // declaring `debug()` prints through it. `String(payload)` printed
  // `payload: secret` where Rust prints `payload: "secret"`, and
  // `[object Object]` for anything the fallback reached (F7). The one shape it
  // cannot get right is a `char` payload, which is a one-character string here
  // and prints `"a"` where Rust prints `'a'`; the emitter reports a provided
  // generic type instantiated with `char` at the type position instead.
  debug(): string {
    return `Attested { payload: ${debugValue(this.payload)}, attestations: ${this.attestations.debug()} }`;
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

  // ── JSON: derived serde on a struct with named fields ──

  toJSON(writePayload: (p: T) => unknown): unknown {
    return { payload: writePayload(this.payload), attestations: this.attestations.toJSON() };
  }

  static fromJson<T>(
    value: unknown,
    readPayload: (v: unknown) => Result<T, JsonError>,
  ): Result<Attested<T>, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `Attested`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('payload' in _o)) {
        return Result.Err(JsonError.custom('missing field `payload`'));
      }
      const _rpayload = readPayload(_o['payload']);
      if (_rpayload.isErr()) return Result.Err(_rpayload.unwrapErr());
      const payload = _rpayload.unwrap();
      $built.push(payload);
      if (!('attestations' in _o)) {
        return Result.Err(JsonError.custom('missing field `attestations`'));
      }
      const _rattestations = AttestationSet.fromJson(_o['attestations']);
      if (_rattestations.isErr()) return Result.Err(_rattestations.unwrapErr());
      const attestations = _rattestations.unwrap();
      $built.push(attestations);
      const $out = new Attested(payload, attestations);
      $kept = true;
      return Result.Ok($out);
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
    }
  }
}
