// MIRRORS: ankurah/proto/src/collection.rs

import { BincodeReader, BincodeWriter } from './codec';

/**
 * CollectionId: newtype wrapper around String.
 * Derived serde — serialized as a bincode String (u64 length + UTF-8 bytes).
 */
export class CollectionId {
  readonly value: string;

  constructor(value: string) {
    this.value = value;
  }

  /** Create from a fixed name (system collections). */
  static fixedName(name: string): CollectionId {
    return new CollectionId(name);
  }

  /** Create from a string. */
  static from(value: string): CollectionId {
    return new CollectionId(value);
  }

  asStr(): string {
    return this.value;
  }

  toString(): string {
    return this.value;
  }

  equals(other: CollectionId): boolean {
    return this.value === other.value;
  }

  equalsStr(other: string): boolean {
    return this.value === other;
  }

  // ── Bincode ──

  encode(writer: BincodeWriter): void {
    writer.writeString(this.value);
  }

  static decode(reader: BincodeReader): CollectionId {
    return new CollectionId(reader.readString());
  }
}
