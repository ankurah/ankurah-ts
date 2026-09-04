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
    return new CollectionId(name);
  }

  asStr(): string {
    return this._0;
  }

  static from(val: string): CollectionId {
    return new CollectionId(val);
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
    let c = this._0 < other._0 ? -1 : this._0 > other._0 ? 1 : 0;
    if (c !== 0) return c;
    return 0;
  }

  clone(): CollectionId {
    return new CollectionId(this._0);
  }

  debug(): string {
    return `CollectionId(${JSON.stringify(this._0)})`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeString(this._0);
  }

  static decode(reader: BincodeReader): CollectionId {
    const _0 = reader.readString();
    return new CollectionId(_0);
  }
}

export function String_fromCollectionId(self: string, collectionId: CollectionId): string {
  try {
    return collectionId._0;
  } finally {
    collectionId.drop();
  }
}

