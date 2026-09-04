// MIRRORS: ankurah/proto/src/collection.rs
import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

export class CollectionId extends Struct {
  _0: string;

  constructor(_0: string) {
    super();
    this._0 = _0;
  }

  static fixedName(name: string): CollectionId {
    return new CollectionId(name.toString());
  }

  asStr(): string {
    return this._0;
  }

  static from(val: string): CollectionId {
    return new CollectionId(val.toString());
  }

  equalsStr(other: string): boolean {
    return this._0 === other;
  }

  asRef(): string {
    return this._0;
  }

  toString(): string {
    return `${this._0}`;
  }

  equals(other: CollectionId): boolean {
    if (this._0 !== other._0) return false;
    return true;
  }

  compareTo(other: CollectionId): number {
    throw new Error('TODO');
  }

  clone(): CollectionId {
    return new CollectionId(this._0);
  }

  encode(writer: BincodeWriter): void {
    writer.writeString(this._0);
  }

  static decode(reader: BincodeReader): CollectionId {
    const _0 = reader.readString();
    return new CollectionId(_0);
  }
}

