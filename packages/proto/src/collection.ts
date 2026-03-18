// MIRRORS: ankurah/proto/src/collection.rs

import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

export class CollectionId extends Struct {
  readonly value: string;

  constructor(value: string) {
    super();
    this.value = value;
  }

  // Rust: fn fixed_name
  // impl CollectionId
  static fixedName(name: string): CollectionId {
    return new CollectionId(name);
  }

  // Rust: fn from (From<&str> / From<String> for CollectionId)
  // impl From<&str> / From<String> for CollectionId
  static from(value: string): CollectionId {
    return new CollectionId(value);
  }

  // Rust: fn eq (PartialEq<str> for CollectionId)
  // impl PartialEq<str> for CollectionId
  equalsStr(other: string): boolean {
    return this.value === other;
  }

  // Rust: fn as_ref (AsRef<str> for CollectionId)
  // Rust: fn as_str
  // impl AsRef<str> / as_str
  asStr(): string {
    return this.value;
  }

  // Rust: derive(PartialEq)
  // impl PartialEq
  equals(other: CollectionId): boolean {
    return this.value === other.value;
  }

  // Rust: fn fmt (Display for CollectionId)
  // impl Display
  toString(): string {
    return this.value;
  }

  // Rust: fn from (From<CollectionId> for String) — SKIP: covered by toString()

  // ── Bincode: derived serde — String (u64 length + UTF-8 bytes) ──

  encode(writer: BincodeWriter): void {
    writer.writeString(this.value);
  }

  static decode(reader: BincodeReader): CollectionId {
    return new CollectionId(reader.readString());
  }
}
