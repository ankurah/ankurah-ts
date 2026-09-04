// PROVIDED: Hand-written Clock — complex binary search, iterator patterns, TryInto impls.
// The transpiler never overwrites this file. Generated clock.ts re-exports this type.

import { Result, Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { EventId, deserialized, jsonString } from './id.provided';
import { DecodeError } from './error';

export class Clock extends Struct {
  private _0: EventId[];

  private constructor(ids: EventId[]) {
    super();
    this._0 = ids;
  }

  static new(ids: EventId[]): Clock {
    return new Clock([...ids]);
  }

  asSlice(): readonly EventId[] {
    return this._0;
  }

  toStrings(): string[] {
    return this._0.map(id => id.toBase64());
  }

  toBase64Short(): string {
    return `[${this._0.map(id => id.toBase64Short()).join(',')}]`;
  }

  toBase64(): string {
    return `[${this._0.map(id => id.toBase64()).join(',')}]`;
  }

  static fromStrings(strings: string[]): Clock {
    const ids: EventId[] = [];
    try {
      for (const s of strings) ids.push(EventId.fromBase64(s));
    } catch (e) {
      // Rust's `?` returns before the half-built Vec is named, and drops it. The ids
      // decoded so far have no other owner, so release them before leaving.
      for (const built of ids) built.drop();
      // Divergence kept from the original hand port: Rust's `?` propagates the id's own
      // DecodeError, where this collapses every one of them to InvalidFormat.
      if (e instanceof DecodeError) throw DecodeError.invalidFormat();
      throw e;
    }
    ids.sort((a, b) => a.compareTo(b));
    return new Clock(ids);
  }

  contains(id: EventId): boolean {
    return this.binarySearch(id) >= 0;
  }

  insert(id: EventId): void {
    const idx = this.binarySearchInsert(id);
    if (idx < this._0.length && this._0[idx].equals(id)) {
      return;
    }
    this._0.splice(idx, 0, id);
  }

  withEvent(id: EventId): Clock {
    const n = this.clone();
    n.insert(id);
    return n;
  }

  get length(): number {
    return this._0.length;
  }

  len(): number {
    return this._0.length;
  }

  isEmpty(): boolean {
    return this._0.length === 0;
  }

  iter(): EventId[] {
    return [...this._0];
  }

  [Symbol.iterator](): Iterator<EventId> {
    return this._0[Symbol.iterator]();
  }

  // Rust: `self.0.clone()` — the caller owns the ids it gets back, so they are cloned
  // rather than aliased; handing out the clock's own ids would make two owners of each.
  toVec(): EventId[] {
    return this._0.map(id => id.clone());
  }

  static from(ids: EventId[]): Clock {
    return new Clock([...ids]);
  }

  static fromByteVecs(idBytes: Uint8Array[]): Clock {
    const ids: EventId[] = [];
    for (const bytes of idBytes) {
      if (bytes.length !== 32) throw DecodeError.invalidLength();
      ids.push(EventId.fromBytes(bytes));
    }
    return new Clock(ids);
  }

  static fromEventId(id: EventId): Clock {
    return new Clock([id]);
  }

  static default(): Clock {
    return new Clock([]);
  }

  static empty(): Clock {
    return Clock.default();
  }

  equals(other: Clock): boolean {
    if (this._0.length !== other._0.length) return false;
    for (let i = 0; i < this._0.length; i++) {
      if (!this._0[i].equals(other._0[i])) return false;
    }
    return true;
  }

  toString(): string {
    return this.toBase64();
  }

  // derive(Clone) clones each element; sharing the id objects would give the original
  // and the copy the same owned values, and dropping both would drop each id twice.
  clone(): Clock {
    return new Clock(this._0.map(id => id.clone()));
  }

  private binarySearch(id: EventId): number {
    let lo = 0;
    let hi = this._0.length - 1;
    while (lo <= hi) {
      const mid = (lo + hi) >>> 1;
      const cmp = this._0[mid].compareTo(id);
      if (cmp === 0) return mid;
      if (cmp < 0) lo = mid + 1;
      else hi = mid - 1;
    }
    return -1;
  }

  private binarySearchInsert(id: EventId): number {
    let lo = 0;
    let hi = this._0.length;
    while (lo < hi) {
      const mid = (lo + hi) >>> 1;
      if (this._0[mid].compareTo(id) < 0) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  }

  // ── JSON: derive(Serialize/Deserialize) ──
  // Clock is a newtype over Vec<EventId>, and serde looks straight through a newtype, so
  // the JSON is the array of the ids' own JSON form — not `toBase64()`, which is the
  // bracketed display string. Deserialization does not sort; only `fromStrings` does.

  toJSON(): string[] {
    return this._0.map(id => id.toJSON());
  }

  static fromJson(value: unknown): Result<Clock, DecodeError> {
    return deserialized(() => {
      if (!Array.isArray(value)) throw DecodeError.invalidFormat();
      const ids: EventId[] = [];
      try {
        for (const item of value) ids.push(EventId.fromBase64(jsonString(item)));
      } catch (e) {
        // Rust returns before the half-built Vec is named, and drops it. The ids decoded
        // so far have no other owner, so release them before the failure carries on out.
        for (const built of ids) built.drop();
        throw e;
      }
      return new Clock(ids);
    });
  }

  encode(writer: BincodeWriter): void {
    writer.writeLength(this._0.length);
    for (const id of this._0) {
      id.encode(writer);
    }
  }

  static decode(reader: BincodeReader): Clock {
    const len = reader.readLength();
    const ids: EventId[] = [];
    for (let i = 0; i < len; i++) {
      ids.push(EventId.decode(reader));
    }
    return new Clock(ids);
  }
}
