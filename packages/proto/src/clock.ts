// MIRRORS: ankurah/proto/src/clock.rs

import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { EventId } from './id';
import { DecodeError } from './error';

/// Set of event ids which represents a head in a DAG of events
export class Clock extends Struct {
  private _0: EventId[];

  private constructor(ids: EventId[]) {
    super();
    this._0 = ids;
  }

  // impl Clock

  // Rust: fn new
  static new(ids: EventId[]): Clock {
    return new Clock([...ids]);
  }

  // Rust: fn as_slice
  asSlice(): readonly EventId[] {
    return this._0;
  }

  // Rust: fn to_strings
  toStrings(): string[] {
    return this._0.map(id => id.toBase64());
  }

  // Rust: fn to_base64_short
  toBase64Short(): string {
    return `[${this._0.map(id => id.toBase64Short()).join(',')}]`;
  }

  // Rust: fn to_base64
  toBase64(): string {
    return `[${this._0.map(id => id.toBase64()).join(',')}]`;
  }

  // Rust: fn from_strings
  static fromStrings(strings: string[]): Clock {
    const ids = strings.map(s => {
      try {
        return EventId.fromBase64(s);
      } catch {
        throw DecodeError.invalidFormat();
      }
    });
    ids.sort((a, b) => a.compareTo(b));
    return new Clock(ids);
  }

  // Rust: fn contains
  contains(id: EventId): boolean {
    return this.binarySearch(id) >= 0;
  }

  // Rust: fn insert
  insert(id: EventId): void {
    const idx = this.binarySearchInsert(id);
    if (idx < this._0.length && this._0[idx].equals(id)) {
      return; // Already present
    }
    this._0.splice(idx, 0, id);
  }

  // Rust: fn with_event
  /// Creates a clone of the clock with the given event inserted
  withEvent(id: EventId): Clock {
    const clone = new Clock([...this._0]);
    clone.insert(id);
    return clone;
  }

  get length(): number {
    return this._0.length;
  }

  /// Rust: fn len(&self) -> usize
  len(): number {
    return this._0.length;
  }

  // Rust: fn is_empty
  isEmpty(): boolean {
    return this._0.length === 0;
  }

  /// Rust: fn iter(&self) -> impl Iterator<Item = &EventId>
  iter(): EventId[] {
    return [...this._0];
  }

  [Symbol.iterator](): Iterator<EventId> {
    return this._0[Symbol.iterator]();
  }

  // Rust: fn to_vec
  toVec(): EventId[] {
    return [...this._0];
  }

  // Rust: fn from (From<Vec<EventId>> for Clock)
  // impl From<Vec<EventId>> for Clock
  static from(ids: EventId[]): Clock {
    return new Clock([...ids]);
  }

  // Rust: fn try_into (TryInto<Clock> for Vec<Vec<u8>>)
  // impl TryInto<Clock> for Vec<Vec<u8>>
  static fromByteVecs(idBytes: Uint8Array[]): Clock {
    const ids: EventId[] = [];
    for (const bytes of idBytes) {
      if (bytes.length !== 32) throw DecodeError.invalidLength();
      ids.push(EventId.fromBytes(bytes));
    }
    return new Clock(ids);
  }

  // Rust: fn from (From<EventId> for Clock)
  // impl From<EventId> for Clock
  static fromEventId(id: EventId): Clock {
    return new Clock([id]);
  }

  // impl Default
  static default(): Clock {
    return new Clock([]);
  }

  static empty(): Clock {
    return Clock.default();
  }

  // Rust: derive(PartialEq)
  // impl PartialEq
  equals(other: Clock): boolean {
    if (this._0.length !== other._0.length) return false;
    for (let i = 0; i < this._0.length; i++) {
      if (!this._0[i].equals(other._0[i])) return false;
    }
    return true;
  }

  // Rust: fn fmt (Display for Clock)
  // impl Display
  toString(): string {
    return this.toBase64();
  }

  // Rust: fn from (From<&Clock> for Vec<EventId>) — SKIP: covered by toVec()

  // ── Binary search helpers ──

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

  // ── Bincode: derived serde — Vec<EventId> ──
  // Each EventId is custom serde (raw 32 bytes), so this is u64 length + N*32 bytes.

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
