// TS-ONLY: bincode reader/writer (per domcorder patterns)
//
// Implements bincode 1.3.x legacy default format (little-endian, fixed-size u64 lengths,
// u32 enum variants, no varint). This is the authoritative TypeScript bincode codec for
// ankurah-ts, following patterns from domcorder/proto-ts but adapted for LE byte order.

import { decodeUtf8 } from '@ankurah/base';

// ─── BincodeWriter ──────────────────────────────────────────────────────────

const enc = new TextEncoder();

export class BincodeWriter {
  private buf: Uint8Array;
  private view: DataView;
  private pos: number = 0;

  constructor(initialCapacity: number = 256) {
    this.buf = new Uint8Array(initialCapacity);
    this.view = new DataView(this.buf.buffer);
  }

  private ensure(bytes: number): void {
    while (this.pos + bytes > this.buf.length) {
      const newBuf = new Uint8Array(this.buf.length * 2);
      newBuf.set(this.buf);
      this.buf = newBuf;
      this.view = new DataView(this.buf.buffer);
    }
  }

  /** Write a single byte (u8). */
  writeBool(v: boolean): void {
    this.ensure(1);
    this.buf[this.pos++] = v ? 1 : 0;
  }

  /** Write a single unsigned byte (u8). */
  writeU8(v: number): void {
    this.ensure(1);
    this.buf[this.pos++] = v & 0xff;
  }

  /** Write a single signed byte (i8). */
  writeI8(v: number): void {
    this.ensure(1);
    this.view.setInt8(this.pos, v);
    this.pos += 1;
  }

  /** Write u16 in little-endian. */
  writeU16(v: number): void {
    this.ensure(2);
    this.view.setUint16(this.pos, v, true);
    this.pos += 2;
  }

  /** Write i16 in little-endian. */
  writeI16(v: number): void {
    this.ensure(2);
    this.view.setInt16(this.pos, v, true);
    this.pos += 2;
  }

  /** Write u32 in little-endian. */
  writeU32(v: number): void {
    this.ensure(4);
    this.view.setUint32(this.pos, v, true);
    this.pos += 4;
  }

  /** Write i32 in little-endian. */
  writeI32(v: number): void {
    this.ensure(4);
    this.view.setInt32(this.pos, v, true);
    this.pos += 4;
  }

  /** Write u64 in little-endian. Accepts bigint. */
  writeU64(v: bigint): void {
    this.ensure(8);
    this.view.setBigUint64(this.pos, v, true);
    this.pos += 8;
  }

  /** Write i64 in little-endian. Accepts bigint. */
  writeI64(v: bigint): void {
    this.ensure(8);
    this.view.setBigInt64(this.pos, v, true);
    this.pos += 8;
  }

  /** Write f64 in little-endian IEEE 754. */
  writeF64(v: number): void {
    this.ensure(8);
    this.view.setFloat64(this.pos, v, true);
    this.pos += 8;
  }

  /**
   * Write a length as u64. Uses number for lengths (with bounds check).
   * Per architectural-decisions.md: number for length fields, bigint for i64/u64 data values.
   */
  writeLength(v: number): void {
    if (v < 0 || v > Number.MAX_SAFE_INTEGER) {
      throw new RangeError(`Length ${v} out of safe integer range`);
    }
    this.writeU64(BigInt(v));
  }

  /** Write a bincode String: u64 length prefix + UTF-8 bytes. */
  writeString(s: string): void {
    const bytes = enc.encode(s);
    this.writeLength(bytes.length);
    this.writeRawBytes(bytes);
  }

  /** Write raw bytes with NO length prefix (for fixed-size arrays). */
  writeRawBytes(bytes: Uint8Array): void {
    this.ensure(bytes.length);
    this.buf.set(bytes, this.pos);
    this.pos += bytes.length;
  }

  /** Write Vec<u8>: u64 length prefix + raw bytes. */
  writeByteVec(bytes: Uint8Array): void {
    this.writeLength(bytes.length);
    this.writeRawBytes(bytes);
  }

  /** Write Option<T>: 0x00 for None, 0x01 + value for Some. */
  writeOption<T>(value: T | null | undefined, writeFn: (writer: BincodeWriter, value: T) => void): void {
    if (value === null || value === undefined) {
      this.writeU8(0);
    } else {
      this.writeU8(1);
      writeFn(this, value);
    }
  }

  /** Write enum variant: u32 index. */
  writeVariant(index: number): void {
    this.writeU32(index);
  }

  /**
   * Write Vec<T>: u64 length + elements.
   * The encodeFn is called for each element.
   */
  writeVec<T>(items: T[], encodeFn: (writer: BincodeWriter, item: T) => void): void {
    this.writeLength(items.length);
    for (const item of items) {
      encodeFn(this, item);
    }
  }

  /**
   * Write BTreeMap<String, V>: u64 length + sorted key-value pairs.
   * Keys are sorted in UTF-8 byte order (Rust String Ord).
   */
  writeStringMap<V>(
    map: Map<string, V>,
    encodeValue: (writer: BincodeWriter, value: V) => void,
  ): void {
    // Sort keys in UTF-8 byte order
    const entries = [...map.entries()].sort((a, b) => compareUtf8Bytes(a[0], b[0]));
    this.writeLength(entries.length);
    for (const [key, value] of entries) {
      this.writeString(key);
      encodeValue(this, value);
    }
  }

  /** Return the encoded bytes (trimmed to actual content). */
  finish(): Uint8Array {
    return this.buf.slice(0, this.pos);
  }
}

// ─── BincodeReader ──────────────────────────────────────────────────────────

// Text is decoded by `decodeUtf8`, the one fatal decode in the port. What
// "fatal" has to mean, case by case, is `packages/base/__tests__/utf8.test.ts`;
// why it has to mean that is port/ownership.md, "Text crossing into the port is
// UTF-8". This reader's own job is to turn the refusal into its own error, with
// the offset the bad bytes were read from.

export class BincodeReader {
  private buf: Uint8Array;
  private view: DataView;
  private pos: number = 0;

  constructor(data: Uint8Array) {
    this.buf = data;
    this.view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  }

  /** Number of bytes remaining. */
  get remaining(): number {
    return this.buf.length - this.pos;
  }

  private checkAvailable(n: number): void {
    if (this.remaining < n) {
      throw new Error(`BincodeReader: need ${n} bytes but only ${this.remaining} remain (at offset ${this.pos})`);
    }
  }

  /** Read a bool (1 byte: 0x00 or 0x01). */
  readBool(): boolean {
    this.checkAvailable(1);
    const v = this.buf[this.pos++];
    if (v !== 0 && v !== 1) {
      throw new Error(`BincodeReader: invalid bool byte: 0x${v.toString(16)}`);
    }
    return v === 1;
  }

  /** Read u8. */
  readU8(): number {
    this.checkAvailable(1);
    return this.buf[this.pos++];
  }

  /** Read i8. */
  readI8(): number {
    this.checkAvailable(1);
    const v = this.view.getInt8(this.pos);
    this.pos += 1;
    return v;
  }

  /** Read u16 in little-endian. */
  readU16(): number {
    this.checkAvailable(2);
    const v = this.view.getUint16(this.pos, true);
    this.pos += 2;
    return v;
  }

  /** Read i16 in little-endian. */
  readI16(): number {
    this.checkAvailable(2);
    const v = this.view.getInt16(this.pos, true);
    this.pos += 2;
    return v;
  }

  /** Read u32 in little-endian. */
  readU32(): number {
    this.checkAvailable(4);
    const v = this.view.getUint32(this.pos, true);
    this.pos += 4;
    return v;
  }

  /** Read i32 in little-endian. */
  readI32(): number {
    this.checkAvailable(4);
    const v = this.view.getInt32(this.pos, true);
    this.pos += 4;
    return v;
  }

  /** Read u64 in little-endian. Returns bigint. */
  readU64(): bigint {
    this.checkAvailable(8);
    const v = this.view.getBigUint64(this.pos, true);
    this.pos += 8;
    return v;
  }

  /** Read i64 in little-endian. Returns bigint. */
  readI64(): bigint {
    this.checkAvailable(8);
    const v = this.view.getBigInt64(this.pos, true);
    this.pos += 8;
    return v;
  }

  /** Read f64 in little-endian IEEE 754. */
  readF64(): number {
    this.checkAvailable(8);
    const v = this.view.getFloat64(this.pos, true);
    this.pos += 8;
    return v;
  }

  /**
   * Read a length (u64 stored as number).
   * Per architectural-decisions.md: number for length fields.
   */
  readLength(): number {
    const v = this.readU64();
    if (v > BigInt(Number.MAX_SAFE_INTEGER)) {
      throw new RangeError(`BincodeReader: length ${v} exceeds MAX_SAFE_INTEGER`);
    }
    return Number(v);
  }

  /** Read a bincode String: u64 length prefix + UTF-8 bytes. */
  readString(): string {
    const len = this.readLength();
    this.checkAvailable(len);
    const bytes = this.buf.subarray(this.pos, this.pos + len);
    this.pos += len;
    const text = decodeUtf8(bytes);
    if (text === null) {
      throw new Error(`BincodeReader: ${len} bytes at offset ${this.pos - len} are not valid UTF-8`);
    }
    return text;
  }

  /** Read exactly n raw bytes (no length prefix). For fixed-size arrays. */
  readRawBytes(n: number): Uint8Array {
    this.checkAvailable(n);
    const bytes = this.buf.slice(this.pos, this.pos + n);
    this.pos += n;
    return bytes;
  }

  /** Read Vec<u8>: u64 length prefix + raw bytes. */
  readByteVec(): Uint8Array {
    const len = this.readLength();
    this.checkAvailable(len);
    const bytes = this.buf.slice(this.pos, this.pos + len);
    this.pos += len;
    return bytes;
  }

  /** Read Option<T>: 0x00 for None (returns null), 0x01 + value for Some. */
  readOption<T>(readFn: (reader: BincodeReader) => T): T | null {
    const tag = this.readU8();
    if (tag === 0) return null;
    if (tag === 1) return readFn(this);
    throw new Error(`BincodeReader: invalid Option tag: 0x${tag.toString(16)}`);
  }

  /** Read enum variant index (u32). */
  readVariant(): number {
    return this.readU32();
  }

  /**
   * Read Vec<T>: u64 length + elements.
   * The decodeFn is called for each element.
   */
  readVec<T>(decodeFn: (reader: BincodeReader) => T): T[] {
    const len = this.readLength();
    const result: T[] = [];
    for (let i = 0; i < len; i++) {
      result.push(decodeFn(this));
    }
    return result;
  }

  /**
   * Read BTreeMap<String, V>: u64 length + key-value pairs.
   * Assumes keys arrive sorted (Rust BTreeMap guarantees this).
   */
  readStringMap<V>(decodeValue: (reader: BincodeReader) => V): Map<string, V> {
    const len = this.readLength();
    const map = new Map<string, V>();
    for (let i = 0; i < len; i++) {
      const key = this.readString();
      const value = decodeValue(this);
      map.set(key, value);
    }
    return map;
  }
}

// ─── Utilities ──────────────────────────────────────────────────────────────

/**
 * Compare two strings by UTF-8 byte order, matching Rust's String Ord.
 * Returns negative if a < b, positive if a > b, 0 if equal.
 */
export function compareUtf8Bytes(a: string, b: string): number {
  const ab = enc.encode(a);
  const bb = enc.encode(b);
  const minLen = Math.min(ab.length, bb.length);
  for (let i = 0; i < minLen; i++) {
    if (ab[i] !== bb[i]) return ab[i] - bb[i];
  }
  return ab.length - bb.length;
}

/**
 * Encode interface. Types that can be bincode-encoded implement this.
 */
export interface BincodeEncodable {
  encode(writer: BincodeWriter): void;
}

/**
 * Decode interface. Types that can be bincode-decoded have a static decode method.
 * (This is a documentation convention, not enforceable via TS interfaces for static methods.)
 */
// static decode(reader: BincodeReader): T;

/**
 * Convenience: serialize a value to Uint8Array.
 */
export function serialize(encodeFn: (writer: BincodeWriter) => void): Uint8Array {
  const writer = new BincodeWriter();
  encodeFn(writer);
  return writer.finish();
}

/**
 * Convenience: deserialize from Uint8Array.
 */
export function deserialize<T>(data: Uint8Array, decodeFn: (reader: BincodeReader) => T): T {
  const reader = new BincodeReader(data);
  return decodeFn(reader);
}
