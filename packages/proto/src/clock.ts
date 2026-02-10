// MIRRORS: ankurah/proto/src/clock.rs

import { BincodeReader, BincodeWriter } from './codec';
import { EventId } from './id';
import { DecodeError } from './error';

/**
 * Clock: Set of EventIds representing a head in a DAG of events.
 * Derived serde — serialized as Vec<EventId> (u64 length + elements).
 * Since EventId has custom serde (raw 32 bytes), each element is 32 bytes.
 */
export class Clock {
  /** Sorted array of EventIds. */
  private ids: EventId[];

  private constructor(ids: EventId[]) {
    this.ids = ids;
  }

  static new(ids: EventId[]): Clock {
    return new Clock([...ids]);
  }

  static empty(): Clock {
    return new Clock([]);
  }

  static default(): Clock {
    return new Clock([]);
  }

  static fromEventId(id: EventId): Clock {
    return new Clock([id]);
  }

  asSlice(): readonly EventId[] {
    return this.ids;
  }

  toStrings(): string[] {
    return this.ids.map(id => id.toBase64());
  }

  toBase64Short(): string {
    return `[${this.ids.map(id => id.toBase64Short()).join(',')}]`;
  }

  toBase64(): string {
    return `[${this.ids.map(id => id.toBase64()).join(',')}]`;
  }

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

  contains(id: EventId): boolean {
    return this.binarySearch(id) >= 0;
  }

  insert(id: EventId): void {
    const idx = this.binarySearchInsert(id);
    if (idx < this.ids.length && this.ids[idx].equals(id)) {
      return; // Already present
    }
    this.ids.splice(idx, 0, id);
  }

  /** Creates a clone of the clock with the given event inserted. */
  withEvent(id: EventId): Clock {
    const clone = new Clock([...this.ids]);
    clone.insert(id);
    return clone;
  }

  get length(): number {
    return this.ids.length;
  }

  isEmpty(): boolean {
    return this.ids.length === 0;
  }

  [Symbol.iterator](): Iterator<EventId> {
    return this.ids[Symbol.iterator]();
  }

  toVec(): EventId[] {
    return [...this.ids];
  }

  equals(other: Clock): boolean {
    if (this.ids.length !== other.ids.length) return false;
    for (let i = 0; i < this.ids.length; i++) {
      if (!this.ids[i].equals(other.ids[i])) return false;
    }
    return true;
  }

  toString(): string {
    return this.toBase64();
  }

  // ── Binary search helpers ──

  private binarySearch(id: EventId): number {
    let lo = 0;
    let hi = this.ids.length - 1;
    while (lo <= hi) {
      const mid = (lo + hi) >>> 1;
      const cmp = this.ids[mid].compareTo(id);
      if (cmp === 0) return mid;
      if (cmp < 0) lo = mid + 1;
      else hi = mid - 1;
    }
    return -1;
  }

  private binarySearchInsert(id: EventId): number {
    let lo = 0;
    let hi = this.ids.length;
    while (lo < hi) {
      const mid = (lo + hi) >>> 1;
      if (this.ids[mid].compareTo(id) < 0) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  }

  // ── Bincode: derived serde — Vec<EventId> ──
  // Each EventId is custom serde (raw 32 bytes), so this is u64 length + N*32 bytes.

  encode(writer: BincodeWriter): void {
    writer.writeLength(this.ids.length);
    for (const id of this.ids) {
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
