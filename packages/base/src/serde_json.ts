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
