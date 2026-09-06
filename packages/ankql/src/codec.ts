// TS-ONLY: bincode reader/writer for @ankurah/ankql
//
// Duplicated from @ankurah/proto/src/codec.ts to avoid circular dependency
// (proto depends on ankql, so ankql cannot depend on proto).
//
// Implements bincode 1.3.x legacy default format (little-endian, fixed-size u64 lengths,
// u32 enum variants, no varint).

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

  /** Return the encoded bytes (trimmed to actual content). */
  finish(): Uint8Array {
    return this.buf.slice(0, this.pos);
  }
}

// ─── BincodeReader ──────────────────────────────────────────────────────────

// A Rust `String` is UTF-8 by construction, so a byte run that is not valid
// UTF-8 could not have come from one: `serde`'s own decoder errors there. A
// non-fatal `TextDecoder` answers U+FFFD instead, which is a different string
// that then flows on as though it had been read — a silent corruption where
// Rust reports. Fatal, and the exception is turned into this codec's own error.
const dec = new TextDecoder('utf-8', { fatal: true });

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
    try {
      return dec.decode(bytes);
    } catch {
      throw new Error(`BincodeReader: ${len} bytes at offset ${this.pos - len} are not valid UTF-8`);
    }
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
}

// ─── ULID utilities ─────────────────────────────────────────────────────────
// Rust's Ulid with derived serde serializes as a 26-char Crockford Base32 string.
// These helpers convert between 16-byte Uint8Array and that string representation.

const CROCKFORD_CHARS = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
const CROCKFORD_DECODE = new Map<string, number>();
for (let i = 0; i < CROCKFORD_CHARS.length; i++) {
  CROCKFORD_DECODE.set(CROCKFORD_CHARS[i], i);
  CROCKFORD_DECODE.set(CROCKFORD_CHARS[i].toLowerCase(), i);
}
// Additional Crockford aliases
CROCKFORD_DECODE.set('O', 0);
CROCKFORD_DECODE.set('o', 0);
CROCKFORD_DECODE.set('I', 1);
CROCKFORD_DECODE.set('i', 1);
CROCKFORD_DECODE.set('L', 1);
CROCKFORD_DECODE.set('l', 1);

export function ulidBytesToString(bytes: Uint8Array): string {
  let value = 0n;
  for (let i = 0; i < 16; i++) {
    value = (value << 8n) | BigInt(bytes[i]);
  }
  const chars: string[] = new Array(26);
  for (let i = 25; i >= 0; i--) {
    chars[i] = CROCKFORD_CHARS[Number(value & 0x1fn)];
    value >>= 5n;
  }
  return chars.join('');
}

export function ulidStringToBytes(str: string): Uint8Array {
  if (str.length !== 26) {
    throw new Error(`Invalid ULID string length: ${str.length} (expected 26)`);
  }
  let value = 0n;
  for (let i = 0; i < 26; i++) {
    const v = CROCKFORD_DECODE.get(str[i]);
    if (v === undefined) {
      throw new Error(`Invalid ULID character: '${str[i]}' at position ${i}`);
    }
    value = (value << 5n) | BigInt(v);
  }
  const bytes = new Uint8Array(16);
  for (let i = 15; i >= 0; i--) {
    bytes[i] = Number(value & 0xffn);
    value >>= 8n;
  }
  return bytes;
}
