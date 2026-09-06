// PROVIDED: Hand-written implementations for types with custom serde or external dependencies.
// The transpiler never overwrites this file. Generated code re-exports these types.
//
// EntityId — 16-byte UUID-like identifier wrapping a ULID.
//   Custom bincode serde: raw 16 bytes, no length prefix.
//
// EventId — defined in data.rs in Rust, co-located here to share base64/ULID utilities.
//   Custom bincode serde: raw 32 bytes, no length prefix.
//
// TransactionId, RequestId, QueryId, UpdateId — ULID wrappers.
//   Derived serde on Ulid: serialized as 26-char string (u64 length + 26 ASCII bytes in bincode).
//
// JSON convention: Rust's serde picks a different representation for human-readable
// formats, so every id here carries both halves of that split. `toJSON()` is what
// `Serialize` writes when the format is human-readable — `JSON.stringify` calls it, so
// the emitter's `serde_json::to_string(&x)` → `JSON.stringify(x)` needs nothing else.
// `fromJson(value)` is `Deserialize` for the same case, and returns the `Result` Rust
// returns so `serde_json::from_str::<T>(&s).unwrap()` → `T.fromJson(JSON.parse(s)).unwrap()`.
// `encode`/`decode` below stay the binary half and are untouched by any of this.

import { JsonError, Result, Struct } from '@ankurah/base';
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

// 0xff marks "not in the alphabet", so a character Rust's URL_SAFE_NO_PAD engine
// rejects — '=', '+', '/', anything outside base64url — is told apart from 'A', which
// is 0. Without the sentinel every stray character silently decoded to a zero byte.
const BASE64URL_DECODE = new Uint8Array(128).fill(0xff);
for (let i = 0; i < BASE64URL_CHARS.length; i++) {
  BASE64URL_DECODE[BASE64URL_CHARS.charCodeAt(i)] = i;
}

function base64urlSextet(str: string, i: number): number {
  const code = str.charCodeAt(i);
  const v = code < 128 ? BASE64URL_DECODE[code] : 0xff;
  if (v === 0xff) throw DecodeError.invalidBase64(`invalid symbol '${str[i]}' at offset ${i}`);
  return v;
}

/** Rust: `general_purpose::URL_SAFE_NO_PAD.decode` — throws `DecodeError` on input it rejects. */
function base64urlDecode(str: string): Uint8Array {
  const len = str.length;
  // A trailing group of one character carries 6 bits and encodes no whole byte, so
  // no unpadded base64 string has this length.
  if (len % 4 === 1) throw DecodeError.invalidBase64(`invalid length ${len}`);
  const outLen = Math.floor((len * 3) / 4);
  const out = new Uint8Array(outLen);
  let j = 0;
  for (let i = 0; i < len; i += 4) {
    const c0 = base64urlSextet(str, i);
    const c1 = i + 1 < len ? base64urlSextet(str, i + 1) : 0;
    const c2 = i + 2 < len ? base64urlSextet(str, i + 2) : 0;
    const c3 = i + 3 < len ? base64urlSextet(str, i + 3) : 0;
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
 * The `u128` a ULID's 16 bytes are, as Rust's derived `Debug` prints it.
 *
 * `ulid::Ulid` is `#[derive(Debug)] pub struct Ulid(pub u128)`, so
 * `#[derive(Debug)]` on a wrapper over one prints `Wrapper(Ulid(<decimal>))`.
 * The bytes stored here are that `u128` big-endian — `Ulid::to_bytes` is
 * `self.0.to_be_bytes()` — so reading them most significant first is the same
 * number.
 */
function ulidDebug(bytes: Uint8Array): string {
  let n = 0n;
  for (const b of bytes) n = (n << 8n) | BigInt(b);
  return `Ulid(${n})`;
}

function ulidBytesToString(bytes: Uint8Array): string {
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

/** Rust: `Ulid::from_string` — throws `DecodeError` on input it rejects. */
function ulidStringToBytes(str: string): Uint8Array {
  if (str.length !== 26) {
    throw new DecodeError('InvalidUlid', `Invalid ULID string length: ${str.length} (expected 26)`);
  }
  let value = 0n;
  for (let i = 0; i < 26; i++) {
    const v = CROCKFORD_DECODE.get(str[i]);
    if (v === undefined) {
      throw new DecodeError('InvalidUlid', `Invalid ULID character: '${str[i]}' at position ${i}`);
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

// ─── Human-readable deserialization ─────────────────────────────────────────

/**
 * Stands in for `?` inside a hand-written `Deserialize`: the first `DecodeError` raised
 * on the way to a value becomes the `Err`. Every step a Deserialize body takes throws
 * rather than returning its own Result, so one call builds exactly one Result — the one
 * the caller receives — and a sequence that fails on its tenth element leaves none
 * behind for the leak registry to collect.
 *
 * The `Err` is a `JsonError`, because a `Deserialize` impl fails with the format's own
 * error type. Rust writes `EntityId::from_base64(s).map_err(serde::de::Error::custom)`,
 * and `custom` keeps the rendered text of the id's error and nothing else — so the
 * `DecodeError` does not cross, and a caller that needs its kind calls `fromBase64`
 * directly, as Rust's non-serde callers call `from_base64`.
 *
 * A `JsonError` is a tracked value: whoever takes one out of the `Result` owns it.
 */
function deserialized<T>(build: () => T): Result<T, JsonError> {
  try {
    return Result.Ok(build());
  } catch (e) {
    // Only a decode failure is an error value. Anything else — an ownership fatal above
    // all — says the emitted code is wrong, and must reach the caller as a throw.
    if (e instanceof DecodeError) return Result.Err(JsonError.custom(e.message));
    throw e;
  }
}

/** Rust: `String::deserialize(deserializer)?` — every id reads a string first. */
function jsonString(value: unknown): string {
  if (typeof value !== 'string') throw DecodeError.notStringValue();
  return value;
}

// ─── EntityId ───────────────────────────────────────────────────────────────

export class EntityId extends Struct {
  readonly bytes: Uint8Array;

  private constructor(bytes: Uint8Array) {
    super();
    this.bytes = bytes;
  }

  // impl EntityId
  static new(): EntityId {
    return new EntityId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array | number[]): EntityId {
    const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    if (arr.length !== 16) throw DecodeError.invalidLength();
    return new EntityId(new Uint8Array(arr));
  }

  toBytes(): Uint8Array {
    return new Uint8Array(this.bytes);
  }

  static fromBase64(input: string): EntityId {
    const decoded = base64urlDecode(input);
    if (decoded.length !== 16) throw DecodeError.invalidLength();
    return new EntityId(decoded);
  }

  toBase64(): string {
    return base64urlEncode(this.bytes);
  }

  toBase64Short(): string {
    const value = this.toBase64();
    return value.slice(value.length - 6);
  }

  // impl Display for EntityId
  toString(): string {
    return this.toBase64();
  }

  // impl std::fmt::Debug for EntityId — `write!(f, "{}", self.to_base64())`
  debug(): string {
    return this.toBase64();
  }

  // impl Ord for EntityId
  //
  // The derive orders the inner Ulid, and Ulid's Ord is over its u128. The 16 bytes
  // stored here are that u128 big-endian — `Ulid::to_bytes` is `self.0.to_be_bytes()`
  // — so comparing them in order, most significant first, is the same comparison.
  compareTo(other: EntityId): number {
    for (let i = 0; i < 16; i++) {
      if (this.bytes[i] !== other.bytes[i]) return this.bytes[i] - other.bytes[i];
    }
    return 0;
  }

  // impl PartialEq for EntityId
  equals(other: EntityId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  clone(): EntityId {
    return EntityId.fromBytes(new Uint8Array(this.bytes));
  }

  // impl Default for EntityId
  static default(): EntityId {
    return EntityId.new();
  }

  // ── JSON: custom serde — base64url, no padding ──

  // impl Serialize for EntityId (human-readable branch)
  toJSON(): string {
    return this.toBase64();
  }

  // impl<'de> Deserialize<'de> for EntityId (human-readable branch)
  static fromJson(value: unknown): Result<EntityId, JsonError> {
    return deserialized(() => EntityId.fromBase64(jsonString(value)));
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
// Divergence: EventId is defined in data.rs in Rust but co-located here in TS
// to share base64/ULID utilities and avoid circular dependencies [E4]

export class EventId extends Struct {
  readonly bytes: Uint8Array;

  private constructor(bytes: Uint8Array) {
    super();
    this.bytes = bytes;
  }

  // impl EventId
  static fromParts(entityId: EntityId, operations: { encode(w: BincodeWriter): void }, parent: { encode(w: BincodeWriter): void }): EventId {
    const { createHash } = require('crypto') as typeof import('crypto');
    const w = new BincodeWriter();
    entityId.encode(w);
    operations.encode(w);
    parent.encode(w);
    const hash = createHash('sha256').update(w.finish()).digest();
    return new EventId(new Uint8Array(hash));
  }

  toBase64(): string {
    return base64urlEncode(this.bytes);
  }

  toBase64Short(): string {
    const value = this.toBase64();
    return value.slice(value.length - 6);
  }

  static fromBase64(input: string): EventId {
    const decoded = base64urlDecode(input);
    if (decoded.length !== 32) throw DecodeError.invalidLength();
    return new EventId(decoded);
  }

  toBytes(): Uint8Array {
    return new Uint8Array(this.bytes);
  }

  static fromBytes(bytes: Uint8Array | number[]): EventId {
    const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    if (arr.length !== 32) throw DecodeError.invalidLength();
    return new EventId(new Uint8Array(arr));
  }

  asBytes(): Uint8Array {
    return this.bytes;
  }

  // impl Display for EventId
  toString(): string {
    return this.toBase64();
  }

  // impl std::fmt::Debug for EventId — `write!(f, "EventId({})", self.to_base64())`
  debug(): string {
    return `EventId(${this.toBase64()})`;
  }

  // impl Ord for EventId
  compareTo(other: EventId): number {
    for (let i = 0; i < 32; i++) {
      if (this.bytes[i] !== other.bytes[i]) return this.bytes[i] - other.bytes[i];
    }
    return 0;
  }

  // impl PartialEq for EventId
  equals(other: EventId): boolean {
    for (let i = 0; i < 32; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  clone(): EventId {
    return EventId.fromBytes(new Uint8Array(this.bytes));
  }

  // ── JSON: custom serde — base64url, no padding ──

  // impl Serialize for EventId (human-readable branch)
  toJSON(): string {
    return this.toBase64();
  }

  // impl<'de> Deserialize<'de> for EventId (human-readable branch)
  static fromJson(value: unknown): Result<EventId, JsonError> {
    return deserialized(() => EventId.fromBase64(jsonString(value)));
  }

  // ── Bincode: custom serde — raw 32 bytes ──

  encode(writer: BincodeWriter): void {
    writer.writeRawBytes(this.bytes);
  }

  static decode(reader: BincodeReader): EventId {
    const bytes = reader.readRawBytes(32);
    return new EventId(new Uint8Array(bytes));
  }
}

// ─── ULID wrapper IDs (derived serde) ───────────────────────────────────────
// Divergence: TransactionId (transaction.rs), RequestId (request.rs), QueryId (subscription.rs),
// UpdateId (update.rs) are each in separate Rust files but co-located here
// to share ULID utilities. Re-exported from their respective generated TS module files. [E4]
// serde: Ulid serialized as 26-char Crockford Base32 string. The ulid crate writes that
// string whatever the format asks for, so unlike EntityId/EventId these four have one
// representation: in bincode, u64 length (26) + 26 ASCII bytes = 34 bytes total; in JSON,
// the bare string. A newtype struct is transparent to serde, so the wrapper adds nothing.

export class TransactionId extends Struct {
  readonly _0: Uint8Array; // 16-byte ULID

  private constructor(_0: Uint8Array) {
    super();
    this._0 = _0;
  }

  static new(): TransactionId {
    return new TransactionId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array): TransactionId {
    if (bytes.length !== 16) throw DecodeError.invalidLength();
    return new TransactionId(new Uint8Array(bytes));
  }

  toString(): string {
    const idStr = ulidBytesToString(this._0);
    return `T${idStr.slice(20)}`;
  }

  toUlidString(): string {
    return ulidBytesToString(this._0);
  }

  // #[derive(Debug)] on `TransactionId(Ulid)` — rustc prints the name and the inner
  // value's own Debug, and `ulid::Ulid` derives one over its `u128`.
  debug(): string {
    return `TransactionId(${ulidDebug(this._0)})`;
  }

  // impl Ord for TransactionId
  //
  // The derive orders the inner Ulid, and Ulid's Ord is over its u128. The 16 bytes
  // stored here are that u128 big-endian — `Ulid::to_bytes` is `self.0.to_be_bytes()`
  // — so comparing them in order, most significant first, is the same comparison.
  compareTo(other: TransactionId): number {
    for (let i = 0; i < 16; i++) {
      if (this._0[i] !== other._0[i]) return this._0[i] - other._0[i];
    }
    return 0;
  }

  equals(other: TransactionId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this._0[i] !== other._0[i]) return false;
    }
    return true;
  }

  clone(): TransactionId {
    return TransactionId.fromBytes(new Uint8Array(this._0));
  }

  toJSON(): string {
    return ulidBytesToString(this._0);
  }

  static fromJson(value: unknown): Result<TransactionId, JsonError> {
    return deserialized(() => TransactionId.fromBytes(ulidStringToBytes(jsonString(value))));
  }

  encode(writer: BincodeWriter): void {
    writer.writeString(ulidBytesToString(this._0));
  }

  static decode(reader: BincodeReader): TransactionId {
    const str = reader.readString();
    return new TransactionId(ulidStringToBytes(str));
  }
}

export class RequestId extends Struct {
  readonly _0: Uint8Array; // 16-byte ULID

  private constructor(_0: Uint8Array) {
    super();
    this._0 = _0;
  }

  static new(): RequestId {
    return new RequestId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array): RequestId {
    if (bytes.length !== 16) throw DecodeError.invalidLength();
    return new RequestId(new Uint8Array(bytes));
  }

  toString(): string {
    const idStr = ulidBytesToString(this._0);
    return `R${idStr.slice(20)}`;
  }

  toUlidString(): string {
    return ulidBytesToString(this._0);
  }

  // #[derive(Debug)] on `RequestId(Ulid)` — rustc prints the name and the inner
  // value's own Debug, and `ulid::Ulid` derives one over its `u128`.
  debug(): string {
    return `RequestId(${ulidDebug(this._0)})`;
  }

  // impl Ord for RequestId
  //
  // The derive orders the inner Ulid, and Ulid's Ord is over its u128. The 16 bytes
  // stored here are that u128 big-endian — `Ulid::to_bytes` is `self.0.to_be_bytes()`
  // — so comparing them in order, most significant first, is the same comparison.
  compareTo(other: RequestId): number {
    for (let i = 0; i < 16; i++) {
      if (this._0[i] !== other._0[i]) return this._0[i] - other._0[i];
    }
    return 0;
  }

  equals(other: RequestId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this._0[i] !== other._0[i]) return false;
    }
    return true;
  }

  clone(): RequestId {
    return RequestId.fromBytes(new Uint8Array(this._0));
  }

  toJSON(): string {
    return ulidBytesToString(this._0);
  }

  static fromJson(value: unknown): Result<RequestId, JsonError> {
    return deserialized(() => RequestId.fromBytes(ulidStringToBytes(jsonString(value))));
  }

  encode(writer: BincodeWriter): void {
    writer.writeString(ulidBytesToString(this._0));
  }

  static decode(reader: BincodeReader): RequestId {
    const str = reader.readString();
    return new RequestId(ulidStringToBytes(str));
  }
}

export class QueryId extends Struct {
  readonly _0: Uint8Array; // 16-byte ULID

  private constructor(_0: Uint8Array) {
    super();
    this._0 = _0;
  }

  static new(): QueryId {
    return new QueryId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array): QueryId {
    if (bytes.length !== 16) throw DecodeError.invalidLength();
    return new QueryId(new Uint8Array(bytes));
  }

  static test(id: bigint): QueryId {
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
    return `P-${ulidBytesToString(this._0)}`;
  }

  toUlidString(): string {
    return ulidBytesToString(this._0);
  }

  // #[derive(Debug)] on `QueryId(Ulid)` — rustc prints the name and the inner
  // value's own Debug, and `ulid::Ulid` derives one over its `u128`.
  debug(): string {
    return `QueryId(${ulidDebug(this._0)})`;
  }

  // impl Ord for QueryId
  //
  // The derive orders the inner Ulid, and Ulid's Ord is over its u128. The 16 bytes
  // stored here are that u128 big-endian — `Ulid::to_bytes` is `self.0.to_be_bytes()`
  // — so comparing them in order, most significant first, is the same comparison.
  compareTo(other: QueryId): number {
    for (let i = 0; i < 16; i++) {
      if (this._0[i] !== other._0[i]) return this._0[i] - other._0[i];
    }
    return 0;
  }

  equals(other: QueryId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this._0[i] !== other._0[i]) return false;
    }
    return true;
  }

  clone(): QueryId {
    return QueryId.fromBytes(new Uint8Array(this._0));
  }

  toJSON(): string {
    return ulidBytesToString(this._0);
  }

  static fromJson(value: unknown): Result<QueryId, JsonError> {
    return deserialized(() => QueryId.fromBytes(ulidStringToBytes(jsonString(value))));
  }

  encode(writer: BincodeWriter): void {
    writer.writeString(ulidBytesToString(this._0));
  }

  static decode(reader: BincodeReader): QueryId {
    const str = reader.readString();
    return new QueryId(ulidStringToBytes(str));
  }
}

export class UpdateId extends Struct {
  readonly _0: Uint8Array; // 16-byte ULID

  private constructor(_0: Uint8Array) {
    super();
    this._0 = _0;
  }

  static new(): UpdateId {
    return new UpdateId(generateUlidBytes());
  }

  static fromBytes(bytes: Uint8Array): UpdateId {
    if (bytes.length !== 16) throw DecodeError.invalidLength();
    return new UpdateId(new Uint8Array(bytes));
  }

  toString(): string {
    const idStr = ulidBytesToString(this._0);
    return `N${idStr.slice(20)}`;
  }

  toUlidString(): string {
    return ulidBytesToString(this._0);
  }

  // #[derive(Debug)] on `UpdateId(Ulid)` — rustc prints the name and the inner
  // value's own Debug, and `ulid::Ulid` derives one over its `u128`.
  debug(): string {
    return `UpdateId(${ulidDebug(this._0)})`;
  }

  // impl Ord for UpdateId
  //
  // The derive orders the inner Ulid, and Ulid's Ord is over its u128. The 16 bytes
  // stored here are that u128 big-endian — `Ulid::to_bytes` is `self.0.to_be_bytes()`
  // — so comparing them in order, most significant first, is the same comparison.
  compareTo(other: UpdateId): number {
    for (let i = 0; i < 16; i++) {
      if (this._0[i] !== other._0[i]) return this._0[i] - other._0[i];
    }
    return 0;
  }

  equals(other: UpdateId): boolean {
    for (let i = 0; i < 16; i++) {
      if (this._0[i] !== other._0[i]) return false;
    }
    return true;
  }

  clone(): UpdateId {
    return UpdateId.fromBytes(new Uint8Array(this._0));
  }

  toJSON(): string {
    return ulidBytesToString(this._0);
  }

  static fromJson(value: unknown): Result<UpdateId, JsonError> {
    return deserialized(() => UpdateId.fromBytes(ulidStringToBytes(jsonString(value))));
  }

  encode(writer: BincodeWriter): void {
    writer.writeString(ulidBytesToString(this._0));
  }

  static decode(reader: BincodeReader): UpdateId {
    const str = reader.readString();
    return new UpdateId(ulidStringToBytes(str));
  }
}

// Re-export base64, ULID and serde utilities for use by other modules
export { base64urlEncode, base64urlDecode, ulidBytesToString, ulidStringToBytes, generateUlidBytes };
export { deserialized, jsonString };
