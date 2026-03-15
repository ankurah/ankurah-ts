// MIRRORS: ankurah/proto/src/collection.rs

import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

export class CollectionId extends Struct {
  readonly value: string;

  constructor(value: string) {
    super();
    this.value = value;
  }

  // impl CollectionId
  static fixedName(name: string): CollectionId {
    return new CollectionId(name);
  }

  // impl From<&str> / From<String> for CollectionId
  static from(value: string): CollectionId {
    return new CollectionId(value);
  }

  // impl PartialEq<str> for CollectionId
  equalsStr(other: string): boolean {
    return this.value === other;
  }

  // impl AsRef<str> / as_str
  asStr(): string {
    return this.value;
  }

  // impl PartialEq
  equals(other: CollectionId): boolean {
    return this.value === other.value;
  }

  // impl Display
  toString(): string {
    return this.value;
  }

  // ── Bincode: derived serde — String (u64 length + UTF-8 bytes) ──

  encode(writer: BincodeWriter): void {
    writer.writeString(this.value);
  }

  static decode(reader: BincodeReader): CollectionId {
    return new CollectionId(reader.readString());
  }
}
