// TS-ONLY: Stand-in for `serde_json::Error`, the error a JSON decode fails with.
//
// It exists so that an emitted `static fromJson(value)` has something to return
// an `Err` of. A Rust `impl<'de> Deserialize<'de> for T` fails with `D::Error`,
// which for `serde_json` is this type, and it is reachable from every crate the
// port emits — which is what rules out each crate's own decode error. A
// hand-written decoder that already has a richer error of its own converts into
// this one the way Rust does, through `serde::de::Error::custom`, which takes
// anything that renders and keeps only the rendered text.
//
// The whole surface is a message and a position. serde_json carries a `Category`
// as well (Io, Syntax, Data, Eof); nothing in the port branches on one, so it is
// not here, and a decoder that needs to tell two failures apart matches on its
// own error before converting.

import { Struct } from './struct.ts';
import { Result as ResultValue } from './result.ts';

/**
 * `serde_json::Error`.
 *
 * A tracked value, because `serde_json::Error` boxes its contents and so has
 * drop glue: a caller that takes one out of a `Result` owns it and drops it,
 * exactly as Rust drops it at the end of the scope that named it. Nothing
 * inside it has drop glue of its own — it is a string and two numbers — so
 * dropping one releases nothing further.
 */
export class JsonError extends Struct {
  readonly #message: string;
  readonly #line: number;
  readonly #column: number;

  private constructor(message: string, line: number, column: number) {
    super('serde_json::Error');
    this.#message = message;
    this.#line = line;
    this.#column = column;
  }

  /**
   * `serde::de::Error::custom(msg)` — what a `Deserialize` impl calls when its
   * own validation rejects the value it was handed. serde_json builds one with
   * no position, so it renders as the message alone.
   */
  static custom(message: string): JsonError {
    return new JsonError(message, 0, 0);
  }

  /**
   * A failure the parser found at a place in the text. serde_json numbers lines
   * and columns from 1, and renders the position only when it has one — so a
   * `syntax(msg)` with no position renders exactly like a `custom(msg)`, which
   * is what serde_json does with a syntax error it built before it started
   * reading.
   */
  static syntax(message: string, line = 0, column = 0): JsonError {
    return new JsonError(message, line, column);
  }

  /**
   * Wrap what `JSON.parse` threw.
   *
   * DELIBERATE DIFFERENCE: the position is lost. serde_json knows the line and
   * column because it drives the parse; `JSON.parse` is the host's parser and
   * reports what the host chose to report, in text that differs between V8,
   * JavaScriptCore and Hermes. Lifting a position out of that text would make
   * the rendered error depend on which engine ran it, so the message crosses
   * whole and the position stays absent.
   *
   * Anything that is not an `Error` is rendered rather than refused: this runs
   * on the failure path, where throwing a second time would bury the first.
   */
  static fromException(thrown: unknown): JsonError {
    return new JsonError(renderThrown(thrown), 0, 0);
  }

  /** The failure, without the position — serde_json's `ErrorCode` rendered. */
  get message(): string {
    this.assertNotDropped();
    return this.#message;
  }

  /** `Error::line()`. 1-based, or 0 for an error with no place in the text. */
  get line(): number {
    this.assertNotDropped();
    return this.#line;
  }

  /** `Error::column()`. 1-based, or 0 for an error with no place in the text. */
  get column(): number {
    this.assertNotDropped();
    return this.#column;
  }

  /**
   * serde_json's `Display`: the message alone when there is no position, and
   * `"<message> at line L column C"` when there is. serde_json decides on the
   * line alone, so a positioned error always prints both numbers.
   *
   * Deliberately does not assert liveness, for the reason `AnyhowError` and
   * `Enum` do not: rendering a value is what a panic message and a debugger do,
   * and both run precisely when something has already gone wrong.
   */
  override toString(): string {
    if (this.isMoved) return 'serde_json::Error (moved)';
    if (this.isDropped) return 'serde_json::Error (dropped)';
    if (this.#line === 0) return this.#message;
    return `${this.#message} at line ${this.#line} column ${this.#column}`;
  }
}

/**
 * What to call an arbitrary thrown value in a message. Nothing here may throw:
 * it runs while building an error, and a second fault would bury the first.
 */
function renderThrown(value: unknown): string {
  if (value === null || value === undefined) return String(value);
  try {
    if (value instanceof Error) return value.message;
    const own = (value as { toString?: unknown }).toString;
    if (typeof own === 'function' && own !== Object.prototype.toString) return String(value);
    return JSON.stringify(value) ?? String(value);
  } catch {
    return '(unrenderable)';
  }
}

// `use serde_json::Error;` names it `Error`, and `import * as serde_json` makes
// that `serde_json.Error`. An export alias creates no local binding, so `Error`
// inside this module is still the global one.
export { JsonError as Error };

/**
 * `serde_json::Result<T>`, which is `Result<T, serde_json::Error>` and nothing
 * more — a type alias in Rust and a type alias here.
 */
export type Result<T> = ResultValue<T, JsonError>;

// ── The lossless integer layer ──────────────────────────────────────────────
//
// `serde_json` keeps an integer token exactly: `9007199254740993` reads back as
// the `u64` it was written from, and writes back out the same digits.
// `JSON.parse` reads every number as a double, so the same token comes back as
// `9007199254740992` and cannot be recovered. A port that uses `JSON.parse` for
// a `u64` field therefore corrupts data silently in both directions, and can
// emit a token above `u64::MAX` that Rust then refuses to read.
//
// `parse` and `stringify` below are the port's `serde_json::from_str` and
// `to_string`. `parse` is a small recursive-descent reader — no `eval`, no
// `Function` — that hands back an integer token beyond the safe range as a
// `bigint` and everything else as `JSON.parse` would. `stringify` writes a
// `bigint` as a bare integer token, which is what `serde_json` writes.
//
// A field the emitter typed `u64` or `i64` reads a `bigint` and writes one, so
// the round trip is exact whatever the magnitude; a field typed `number` reads
// a JavaScript number, so `1` is `1` and not `1n`.

/** `u64::MAX` and the two `i64` bounds, which are what Rust refuses outside. */
const U64_MAX = 18446744073709551615n;
const I64_MIN = -9223372036854775808n;

/**
 * `serde_json::from_str` for the port: `JSON.parse` with the integer tokens
 * kept.
 *
 * Every integer token outside `Number.MAX_SAFE_INTEGER` comes back as a
 * `bigint`; every other number comes back as a `number`, so nothing that used
 * to be a `number` becomes a `bigint`. Throws a `JsonError` where serde_json
 * would answer `Err`.
 */
export function parse(text: string): ResultValue<unknown, JsonError> {
  const reader = new JsonReader(text);
  try {
    reader.skipWhitespace();
    const value = reader.value();
    reader.skipWhitespace();
    if (!reader.atEnd()) {
      return ResultValue.Err(JsonError.syntax('trailing characters', 1, reader.position + 1));
    }
    return ResultValue.Ok(value);
  } catch (thrown) {
    if (thrown instanceof Fault) {
      return ResultValue.Err(JsonError.syntax(thrown.message, 1, thrown.at + 1));
    }
    throw thrown;
  }
}

/**
 * `serde_json::to_string` for the port: `JSON.stringify` with a `bigint`
 * written as the bare integer token Rust writes, rather than throwing.
 *
 * A `bigint` outside `u64::MAX` or below `i64::MIN` is a value Rust could not
 * have produced and could not read back, so it is an error here rather than a
 * token nothing accepts.
 */
export function stringify(value: unknown): ResultValue<string, JsonError> {
  try {
    return ResultValue.Ok(write(value));
  } catch (thrown) {
    if (thrown instanceof Fault) {
      return ResultValue.Err(JsonError.custom(thrown.message));
    }
    throw thrown;
  }
}

function write(value: unknown): string {
  if (typeof value === 'bigint') {
    return bigintToken(value);
  }
  if (value === null || value === undefined) return 'null';
  if (typeof value === 'number') {
    // Rust has no NaN or infinity in JSON, and serde_json refuses to write one.
    if (!Number.isFinite(value)) {
      throw new Fault(`${value} cannot be written as JSON`, 0);
    }
    return JSON.stringify(value);
  }
  if (typeof value === 'string' || typeof value === 'boolean') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(write).join(',')}]`;
  }
  if (typeof value === 'object') {
    const parts: string[] = [];
    for (const [key, member] of Object.entries(value as Record<string, unknown>)) {
      if (member === undefined) continue;
      parts.push(`${JSON.stringify(key)}:${write(member)}`);
    }
    return `{${parts.join(',')}}`;
  }
  throw new Fault(`a ${typeof value} cannot be written as JSON`, 0);
}

/**
 * What the reader and the writer throw among themselves.
 *
 * NOT a `JsonError`: a `JsonError` is a tracked value, and one built on a path
 * that abandons it is a leak the registry will report. The two public functions
 * catch this and build the tracked error once, on the way out, at the moment
 * something takes ownership of it.
 */
class Fault extends Error {
  readonly at: number;
  constructor(message: string, at: number) {
    super(message);
    this.at = at;
  }
}

function bigintToken(value: bigint): string {
  if (value > U64_MAX || value < I64_MIN) {
    throw new Fault(
      `${value} is outside the range Rust can read back (i64::MIN..=u64::MAX)`,
      0,
    );
  }
  return value.toString();
}

/**
 * The reader `parse` is. Recursive descent over the text, one character at a
 * time; the only thing it does that `JSON.parse` does not is decide, from the
 * TOKEN, whether a number is an integer beyond the safe range.
 */
class JsonReader {
  readonly #text: string;
  #at = 0;

  constructor(text: string) {
    this.#text = text;
  }

  get position(): number {
    return this.#at;
  }

  atEnd(): boolean {
    return this.#at >= this.#text.length;
  }

  skipWhitespace(): void {
    while (this.#at < this.#text.length) {
      const ch = this.#text[this.#at];
      if (ch === ' ' || ch === '\t' || ch === '\n' || ch === '\r') this.#at += 1;
      else break;
    }
  }

  value(): unknown {
    this.skipWhitespace();
    if (this.atEnd()) throw this.fail('unexpected end of input');
    const ch = this.#text[this.#at];
    switch (ch) {
      case '{':
        return this.object();
      case '[':
        return this.array();
      case '"':
        return this.string();
      case 't':
        this.literal('true');
        return true;
      case 'f':
        this.literal('false');
        return false;
      case 'n':
        this.literal('null');
        return null;
      default:
        return this.number();
    }
  }

  private object(): Record<string, unknown> {
    this.#at += 1;
    const out: Record<string, unknown> = {};
    this.skipWhitespace();
    if (this.#text[this.#at] === '}') {
      this.#at += 1;
      return out;
    }
    for (;;) {
      this.skipWhitespace();
      if (this.#text[this.#at] !== '"') throw this.fail('expected a key');
      const key = this.string();
      this.skipWhitespace();
      if (this.#text[this.#at] !== ':') throw this.fail('expected `:`');
      this.#at += 1;
      // `defineProperty`, not assignment: `out['__proto__'] = v` sets the
      // object's PROTOTYPE instead of creating the member, so the key vanished
      // from `hasOwnProperty` and `stringify` wrote the document back without
      // it. serde_json treats `__proto__` as an ordinary key, and so does this.
      Object.defineProperty(out, key, {
        value: this.value(),
        enumerable: true,
        writable: true,
        configurable: true,
      });
      this.skipWhitespace();
      const ch = this.#text[this.#at];
      if (ch === ',') {
        this.#at += 1;
        continue;
      }
      if (ch === '}') {
        this.#at += 1;
        return out;
      }
      throw this.fail('expected `,` or `}`');
    }
  }

  private array(): unknown[] {
    this.#at += 1;
    const out: unknown[] = [];
    this.skipWhitespace();
    if (this.#text[this.#at] === ']') {
      this.#at += 1;
      return out;
    }
    for (;;) {
      out.push(this.value());
      this.skipWhitespace();
      const ch = this.#text[this.#at];
      if (ch === ',') {
        this.#at += 1;
        continue;
      }
      if (ch === ']') {
        this.#at += 1;
        return out;
      }
      throw this.fail('expected `,` or `]`');
    }
  }

  private string(): string {
    const start = this.#at;
    this.#at += 1;
    while (this.#at < this.#text.length) {
      const ch = this.#text[this.#at];
      if (ch === '\\') {
        this.#at += 2;
        continue;
      }
      if (ch === '"') {
        this.#at += 1;
        // The escapes are JSON's own, so the host's reader is what decodes
        // them: writing a second unescaper here would be a second thing to get
        // wrong about `🚀`. What it must not do is THROW past this reader: a
        // `SyntaxError` leaving `parse` is an exception where `from_str` answers
        // `Err`, and seven live boundaries — storage-sqlite's engine, core's
        // system, the value reader — call `parse` for a `Result`.
        const quoted = this.#text.slice(start, this.#at);
        // `JSON.parse` accepts a lone `\uD800` and hands back a string no
        // encoder can write out again; serde_json answers
        // `Err(unexpected end of hex escape)`. The escapes are checked here and
        // decoded by the host, so there is still only one unescaper.
        this.refuseAnUnpairedSurrogate(quoted);
        try {
          return JSON.parse(quoted) as string;
        } catch {
          throw this.fail('invalid string');
        }
      }
      // JSON forbids a raw control character inside a string; serde_json says
      // so by name. `JSON.parse` refuses it too, by throwing.
      if (ch !== undefined && ch < ' ') {
        throw this.fail('control character (\\u0000-\\u001F) found while parsing a string');
      }
      this.#at += 1;
    }
    throw this.fail('unterminated string');
  }

  /**
   * Refuse a `\uD800`-`\uDFFF` escape that is not half of a pair.
   *
   * A surrogate is half a code point. Written alone it is a string JavaScript
   * holds and no UTF-8 encoder can write, so serde_json refuses it at the
   * escape — and `JSON.parse` does not, so the port used to accept a document
   * Rust rejects and then produce text `stringify` could not write back.
   *
   * Only ESCAPED surrogates: a raw one in the source text is already refused by
   * the host's reader, and a well-formed pair is one code point.
   */
  private refuseAnUnpairedSurrogate(quoted: string): void {
    for (let at = 0; at < quoted.length; at++) {
      if (quoted[at] !== '\\') continue;
      if (quoted[at + 1] !== 'u') {
        // Any other escape is two characters; skipping the second keeps a
        // `\\\\` from being read as the start of an escape.
        at += 1;
        continue;
      }
      const code = Number.parseInt(quoted.slice(at + 2, at + 6), 16);
      at += 5;
      if (!Number.isNaN(code) && code >= 0xd800 && code <= 0xdbff) {
        // A high surrogate: the next escape has to be its low half.
        const low = Number.parseInt(quoted.slice(at + 3, at + 7), 16);
        const paired =
          quoted[at + 1] === '\\' &&
          quoted[at + 2] === 'u' &&
          !Number.isNaN(low) &&
          low >= 0xdc00 &&
          low <= 0xdfff;
        if (!paired) throw this.fail('unexpected end of hex escape');
        at += 6;
        continue;
      }
      if (!Number.isNaN(code) && code >= 0xdc00 && code <= 0xdfff) {
        // A low surrogate with no high half in front of it.
        throw this.fail('unexpected end of hex escape');
      }
    }
  }

  private literal(word: string): void {
    if (!this.#text.startsWith(word, this.#at)) throw this.fail(`expected \`${word}\``);
    this.#at += word.length;
  }

  private number(): number | bigint {
    const start = this.#at;
    if (this.#text[this.#at] === '-') this.#at += 1;
    const digitsFrom = this.#at;
    this.digits();
    if (this.#at === digitsFrom) throw this.fail('expected a value');
    // JSON's grammar allows one leading zero and no more: `01` is not a number.
    // serde_json stops after the `0` and calls the rest trailing characters;
    // either way the document is refused, and accepting it read `01` as `1`.
    if (this.#text[digitsFrom] === '0' && this.#at > digitsFrom + 1) {
      throw this.fail('invalid number');
    }
    const integerEnd = this.#at;
    let fractional = false;
    if (this.#text[this.#at] === '.') {
      fractional = true;
      this.#at += 1;
      const from = this.#at;
      this.digits();
      // `1.` has no fraction. `Number('1.')` is 1, so it used to be accepted.
      if (this.#at === from) throw this.fail('invalid number');
    }
    if (this.#text[this.#at] === 'e' || this.#text[this.#at] === 'E') {
      fractional = true;
      this.#at += 1;
      if (this.#text[this.#at] === '+' || this.#text[this.#at] === '-') this.#at += 1;
      const from = this.#at;
      this.digits();
      if (this.#at === from) throw this.fail('invalid number');
    }
    const token = this.#text.slice(start, this.#at);
    if (fractional) {
      const value = Number(token);
      // serde_json refuses a float the format cannot hold: `1e999` is
      // `number out of range`, not `Infinity`.
      if (!Number.isFinite(value)) throw this.fail('number out of range');
      return value;
    }
    // An integer token. It stays a `number` while a `number` can hold it
    // exactly, so nothing that used to be a `number` becomes a `bigint`; beyond
    // that it is a `bigint`, which is what keeps `u64::MAX` readable.
    const asNumber = Number(this.#text.slice(start, integerEnd));
    if (Number.isSafeInteger(asNumber)) return asNumber;
    return BigInt(token);
  }

  /** Walk past a run of decimal digits. */
  private digits(): void {
    while (this.#at < this.#text.length && this.#text[this.#at] >= '0' && this.#text[this.#at] <= '9') {
      this.#at += 1;
    }
  }

  private fail(message: string): Fault {
    return new Fault(message, this.#at);
  }
}

// ── The reader's two combinators ───────────────────────────────────────────
//
// An emitted `fromJson` reads one field at a time and hands an `Err` straight
// out. A field whose type is a LIST or a MAP has many reads inside it, and the
// same rule applies to each: the first failure is the answer, and everything
// already decoded is released before it leaves. Doing that inline would be a
// loop and a `try` per field in every emitted reader; these two are the loop,
// written once.

import { dropOwned } from './object.ts';

/**
 * Every read, or the first failure — with everything already decoded released.
 *
 * `Result<T, JsonError>[]` → `Result<T[], JsonError>`. What Rust's
 * `collect::<Result<Vec<_>, _>>()` does, plus the drop: a decoder owns what it
 * has built until it returns, so the elements before the failing one are this
 * function's to release.
 */
export function jsonAll<T>(reads: ResultValue<T, JsonError>[]): ResultValue<T[], JsonError> {
  const out: T[] = [];
  for (let i = 0; i < reads.length; i += 1) {
    const read = reads[i];
    if (read.isErr()) {
      dropOwned(out);
      // The reads after the failing one succeeded and are owned too.
      for (let j = i + 1; j < reads.length; j += 1) {
        const later = reads[j];
        if (later.isOk()) dropOwned(later.unwrap());
        else later.unwrapErr().drop();
      }
      return ResultValue.Err(read.unwrapErr());
    }
    out.push(read.unwrap());
  }
  return ResultValue.Ok(out);
}

/**
 * `Result::map` for a read: build something out of what came back, or hand the
 * error on untouched.
 */
export function jsonMap<T, U>(
  read: ResultValue<T, JsonError>,
  build: (value: T) => U,
): ResultValue<U, JsonError> {
  if (read.isErr()) return ResultValue.Err(read.unwrapErr());
  return ResultValue.Ok(build(read.unwrap()));
}
