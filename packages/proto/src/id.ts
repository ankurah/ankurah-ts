// MIRRORS: ankurah/proto/src/id.rs
//
// EntityId — 16-byte UUID-like identifier wrapping a ULID.
//   Custom bincode serde: raw 16 bytes, no length prefix.
//   Human-readable serde (JSON): base64url-no-pad string.
//
// TransactionId, RequestId, QueryId, UpdateId — ULID wrappers.
//   Derived serde on Ulid: serialized as 26-char string (u64 length + 26 ASCII bytes in bincode).

import { BincodeReader, BincodeWriter } from './codec';
import { DecodeError } from './error';

// ─── Base64 URL-safe no-pad utilities ───────────────────────────────────────

const BASE64URL_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_';

function base64urlEncode(bytes: Uint8Array): string {
  let result = '';
  const len = bytes.length;
  for (let i = 0; i < len; i += 3) {
    const b0 = bytes[i];
    const b1 = i + 1 < len ? bytes[i + 1] : 0;
    const b2 = i + 2 < len ? bytes[i + 2] : 0;
    result += BASE64URL_CHARS[(b0 >> 2) & 0x3f];
    result += BASE64URL_CHARS[((b0 << 4) | (b1 >> 4)) & 0x3f];
    if (i + 1 < len) {
      result += BASE64URL_CHARS[((b1 << 2) | (b2 >> 6)) & 0x3f];
    }
    if (i + 2 < len) {
      result += BASE64URL_CHARS[b2 & 0x3f];
    }
  }
  return result;
}

const BASE64URL_DECODE = new Uint8Array(128);
for (let i = 0; i < BASE64URL_CHARS.length; i++) {
  BASE64URL_DECODE[BASE64URL_CHARS.charCodeAt(i)] = i;
}

function base64urlDecode(str: string): Uint8Array {
  const len = str.length;
  // Calculate output length accounting for no padding
  const outLen = Math.floor((len * 3) / 4);
  const out = new Uint8Array(outLen);
  let j = 0;
  for (let i = 0; i < len; i += 4) {
    const c0 = BASE64URL_DECODE[str.charCodeAt(i)];
    const c1 = i + 1 < len ? BASE64URL_DECODE[str.charCodeAt(i + 1)] : 0;
    const c2 = i + 2 < len ? BASE64URL_DECODE[str.charCodeAt(i + 2)] : 0;
    const c3 = i + 3 < len ? BASE64URL_DECODE[str.charCodeAt(i + 3)] : 0;
    out[j++] = ((c0 << 2) | (c1 >> 4)) & 0xff;
    if (i + 2 < len) {
      out[j++] = ((c1 << 4) | (c2 >> 2)) & 0xff;
    }
    if (i + 3 < len) {
      out[j++] = ((c2 << 6) | c3) & 0xff;
    }
  }
  return out.slice(0, j);
}

// ─── ULID utilities ─────────────────────────────────────────────────────────

// Crockford's Base32 encoding (used by ULID)
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

/**
 * Generate a new ULID as 16 bytes.
 * ULID = 48-bit timestamp (ms since epoch) + 80-bit random.
 */
function generateUlidBytes(): Uint8Array {
  const bytes = new Uint8Array(16);
  const now = Date.now();
  // Timestamp: 6 bytes (48 bits) in big-endian at bytes[0..6]
  bytes[0] = (now / 0x10000000000) & 0xff;
  bytes[1] = (now / 0x100000000) & 0xff;
  bytes[2] = (now / 0x1000000) & 0xff;
  bytes[3] = (now / 0x10000) & 0xff;
  bytes[4] = (now / 0x100) & 0xff;
  bytes[5] = now & 0xff;
  // Random: 10 bytes
  const random = new Uint8Array(10);
  if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
    crypto.getRandomValues(random);
  } else {
    for (let i = 0; i < 10; i++) random[i] = Math.floor(Math.random() * 256);
  }
  bytes.set(random, 6);
  return bytes;
}

/**
 * Encode 16 ULID bytes as a 26-char Crockford Base32 string.
 * Matches Rust ulid::Ulid::to_string().
 */
function ulidBytesToString(bytes: Uint8Array): string {
  // ULID is 128 bits = 26 Crockford Base32 characters
  // Big-endian: first byte is MSB
  // Convert to a BigInt for easy bit extraction
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

/**
 * Parse a 26-char Crockford Base32 ULID string back to 16 bytes.
 */
function ulidStringToBytes(str: string): Uint8Array {
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

// ─── EntityId ───────────────────────────────────────────────────────────────

/**
 * EntityId: 16-byte identifier wrapping a ULID.
 *
 * Bincode serde: custom — raw 16 bytes, no length prefix.
 * JSON serde: base64url-no-pad string.
 */
export class EntityId {
  /** Raw 16 bytes (ULID in big-endian byte order). */
  readonly bytes: Uint8Array;

  private constructor(bytes: Uint8Array) {
    this.bytes = bytes;
  }

  /** Generate a new random EntityId. */
  static new(): EntityId {
    return new EntityId(generateUlidBytes());
  }

  /** Create from a 16-byte array. */
  static fromBytes(bytes: Uint8Array | number[]): EntityId {
    const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    if (arr.length !== 16) throw DecodeError.invalidLength();
    return new EntityId(new Uint8Array(arr));
  }

  /** Parse from base64url-no-pad string. */
  static fromBase64(input: string): EntityId {
    let decoded: Uint8Array;
    try {
      decoded = base64urlDecode(input);
    } catch {
      throw DecodeError.invalidBase64();
    }
    if (decoded.length !== 16) throw DecodeError.invalidLength();
    return new EntityId(decoded);
  }

  /** Encode as base64url-no-pad string. */
  toBase64(): string {
    return base64urlEncode(this.bytes);
  }

  /** Last 6 characters of the base64 encoding (for short display). */
  toBase64Short(): string {
    const full = this.toBase64();
    return full.slice(full.length - 6);
  }

  /** Convert to 16-byte array. */
  toBytes(): Uint8Array {
    return new Uint8Array(this.bytes);
  }

  /** Display as base64url-no-pad string (matching Rust Display). */
  toString(): string {
    return this.toBase64();
  }

  /** Value equality. */
  equals(other: EntityId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  // ── Bincode: custom serde — raw 16 bytes ──

  encode(writer: BincodeWriter): void {
    writer.writeRawBytes(this.bytes);
  }

  static decode(reader: BincodeReader): EntityId {
    const bytes = reader.readRawBytes(16);
    return new EntityId(new Uint8Array(bytes));
  }
}

// ─── EventId ────────────────────────────────────────────────────────────────

/**
 * EventId: 32-byte hash identifier (SHA-256 of event content).
 *
 * Bincode serde: custom — raw 32 bytes, no length prefix.
 * JSON serde: base64url-no-pad string.
 */
export class EventId {
  /** Raw 32 bytes (SHA-256 hash). */
  readonly bytes: Uint8Array;

  private constructor(bytes: Uint8Array) {
    this.bytes = bytes;
  }

  /** Create from a 32-byte array. */
  static fromBytes(bytes: Uint8Array | number[]): EventId {
    const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    if (arr.length !== 32) throw DecodeError.invalidLength();
    return new EventId(new Uint8Array(arr));
  }

  /** Parse from base64url-no-pad string. */
  static fromBase64(input: string): EventId {
    let decoded: Uint8Array;
    try {
      decoded = base64urlDecode(input);
    } catch {
      throw DecodeError.invalidBase64();
    }
    if (decoded.length !== 32) throw DecodeError.invalidLength();
    return new EventId(decoded);
  }

  /** Encode as base64url-no-pad string. */
  toBase64(): string {
    return base64urlEncode(this.bytes);
  }

  /** Last 6 characters of the base64 encoding (for short display). */
  toBase64Short(): string {
    const full = this.toBase64();
    return full.slice(full.length - 6);
  }

  /** Convert to 32-byte array. */
  toBytes(): Uint8Array {
    return new Uint8Array(this.bytes);
  }

  /** Display as base64url-no-pad string (matching Rust Display). */
  toString(): string {
    return this.toBase64();
  }

  /** Byte-wise comparison for sorting (matches Rust Ord). */
  compareTo(other: EventId): number {
    for (let i = 0; i < 32; i++) {
      if (this.bytes[i] !== other.bytes[i]) return this.bytes[i] - other.bytes[i];
    }
    return 0;
  }

  /** Value equality. */
  equals(other: EventId): boolean {
    for (let i = 0; i < 32; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  // ── Bincode: custom serde — raw 32 bytes ──

  encode(writer: BincodeWriter): void {
    writer.writeRawBytes(this.bytes);
  }

  static decode(reader: BincodeReader): EventId {
    const bytes = reader.readRawBytes(32);
    return new EventId(new Uint8Array(bytes));
  }

  /**
   * Compute an EventId from entity_id, operations, and parent clock.
   *
   * Rust: `pub fn from_parts(entity_id: &EntityId, operations: &OperationSet, parent: &Clock) -> Self`
   * SHA-256 hash of bincode-serialized (entity_id || operations || parent).
   *
   * NOTE: This import is lazy to avoid circular dependencies with data.ts.
   */
  static fromParts(entityId: EntityId, operations: { encode(w: BincodeWriter): void }, parent: { encode(w: BincodeWriter): void }): EventId {
    const { createHash } = require('crypto') as typeof import('crypto');
    const w = new BincodeWriter();
    entityId.encode(w);
    operations.encode(w);
    parent.encode(w);
    const hash = createHash('sha256').update(w.finish()).digest();
    return new EventId(new Uint8Array(hash));
  }
}

// ─── ULID wrapper IDs (derived serde) ───────────────────────────────────────
// TransactionId, RequestId, QueryId, UpdateId all wrap a Ulid and use derived
// serde, which serializes the Ulid as a 26-char Crockford Base32 string.
// In bincode: u64 length (26) + 26 ASCII bytes = 34 bytes total.

/**
 * TransactionId: ULID wrapper for transaction identification.
 */
export class TransactionId {
  readonly bytes: Uint8Array; // 16-byte ULID

  private constructor(bytes: Uint8Array) {
    this.bytes = bytes;
  }

  static new(): TransactionId {
    return new TransactionId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array): TransactionId {
    if (bytes.length !== 16) throw DecodeError.invalidLength();
    return new TransactionId(new Uint8Array(bytes));
  }

  toString(): string {
    const idStr = ulidBytesToString(this.bytes);
    return `T${idStr.slice(20)}`;
  }

  toUlidString(): string {
    return ulidBytesToString(this.bytes);
  }

  equals(other: TransactionId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  // ── Bincode: derived serde on Ulid — 26-char string ──

  encode(writer: BincodeWriter): void {
    writer.writeString(ulidBytesToString(this.bytes));
  }

  static decode(reader: BincodeReader): TransactionId {
    const str = reader.readString();
    return new TransactionId(ulidStringToBytes(str));
  }
}

/**
 * RequestId: ULID wrapper for request identification.
 */
export class RequestId {
  readonly bytes: Uint8Array; // 16-byte ULID

  private constructor(bytes: Uint8Array) {
    this.bytes = bytes;
  }

  static new(): RequestId {
    return new RequestId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array): RequestId {
    if (bytes.length !== 16) throw DecodeError.invalidLength();
    return new RequestId(new Uint8Array(bytes));
  }

  toString(): string {
    const idStr = ulidBytesToString(this.bytes);
    return `R${idStr.slice(20)}`;
  }

  toUlidString(): string {
    return ulidBytesToString(this.bytes);
  }

  equals(other: RequestId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  // ── Bincode: derived serde on Ulid — 26-char string ──

  encode(writer: BincodeWriter): void {
    writer.writeString(ulidBytesToString(this.bytes));
  }

  static decode(reader: BincodeReader): RequestId {
    const str = reader.readString();
    return new RequestId(ulidStringToBytes(str));
  }
}

/**
 * QueryId: ULID wrapper for subscription/query identification.
 */
export class QueryId {
  readonly bytes: Uint8Array; // 16-byte ULID

  private constructor(bytes: Uint8Array) {
    this.bytes = bytes;
  }

  static new(): QueryId {
    return new QueryId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array): QueryId {
    if (bytes.length !== 16) throw DecodeError.invalidLength();
    return new QueryId(new Uint8Array(bytes));
  }

  /** For testing only — matches Rust QueryId::test(id) = Ulid::from_parts(id, 0). */
  static test(id: bigint): QueryId {
    // Ulid::from_parts(timestamp_ms, random) encodes as:
    //   bytes[0..6] = timestamp (48 bits, big-endian)
    //   bytes[6..16] = random (80 bits)
    // from_parts(id as u64, 0) means timestamp = id, random = 0
    const bytes = new Uint8Array(16);
    bytes[0] = Number((id >> 40n) & 0xffn);
    bytes[1] = Number((id >> 32n) & 0xffn);
    bytes[2] = Number((id >> 24n) & 0xffn);
    bytes[3] = Number((id >> 16n) & 0xffn);
    bytes[4] = Number((id >> 8n) & 0xffn);
    bytes[5] = Number(id & 0xffn);
    // bytes[6..16] are already 0
    return new QueryId(bytes);
  }

  toString(): string {
    return `P-${ulidBytesToString(this.bytes)}`;
  }

  toUlidString(): string {
    return ulidBytesToString(this.bytes);
  }

  equals(other: QueryId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  // ── Bincode: derived serde on Ulid — 26-char string ──

  encode(writer: BincodeWriter): void {
    writer.writeString(ulidBytesToString(this.bytes));
  }

  static decode(reader: BincodeReader): QueryId {
    const str = reader.readString();
    return new QueryId(ulidStringToBytes(str));
  }
}

/**
 * UpdateId: ULID wrapper for update identification.
 */
export class UpdateId {
  readonly bytes: Uint8Array; // 16-byte ULID

  private constructor(bytes: Uint8Array) {
    this.bytes = bytes;
  }

  static new(): UpdateId {
    return new UpdateId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array): UpdateId {
    if (bytes.length !== 16) throw DecodeError.invalidLength();
    return new UpdateId(new Uint8Array(bytes));
  }

  toString(): string {
    const idStr = ulidBytesToString(this.bytes);
    return `N${idStr.slice(20)}`;
  }

  toUlidString(): string {
    return ulidBytesToString(this.bytes);
  }

  equals(other: UpdateId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  // ── Bincode: derived serde on Ulid — 26-char string ──

  encode(writer: BincodeWriter): void {
    writer.writeString(ulidBytesToString(this.bytes));
  }

  static decode(reader: BincodeReader): UpdateId {
    const str = reader.readString();
    return new UpdateId(ulidStringToBytes(str));
  }
}

// Re-export base64 utilities for use by other modules
export { base64urlEncode, base64urlDecode, ulidBytesToString, ulidStringToBytes, generateUlidBytes };
