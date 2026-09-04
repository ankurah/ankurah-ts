// TS-ONLY: Rust's Result<T, E> type — an Enum with Ok and Err variants.
//
// Usage mirrors Rust:
//   Result.Ok(value)     — construct an Ok variant
//   Result.Err(error)    — construct an Err variant
//   result.isOk()        — check if Ok
//   result.isErr()       — check if Err
//   result.unwrap()      — extract Ok value or panic
//   result.unwrapErr()   — extract Err value or panic
//   result.match({ Ok: v => ..., Err: e => ... })
//
// Ownership mirrors Rust too: every method Rust declares with `self` consumes
// the Result here, handing the payload to exactly one new owner and leaving the
// original moved. A moved Result is not a leak and is never dropped, and using
// one again is fatal. isOk/isErr take &self in Rust, so they borrow and change
// nothing. Rust's Option<T> is `T | null` in this port, so ok() and err() return
// the payload or null.

import { Enum } from './enum.ts';
import { dropOwned } from './object.ts';

type ResultV<T, E> = {
  Ok: { _0: T };
  Err: { _0: E };
};

/**
 * Render a payload for a panic message, preferring its own toString(). Nothing
 * in here may throw: it runs on the panic path, where a second fault would bury
 * the first one it was called to describe.
 */
function describe(value: unknown): string {
  if (value === null || value === undefined) return String(value);
  try {
    const own = (value as any).toString;
    if (typeof own === 'function' && own !== Object.prototype.toString) return String(value);
    return JSON.stringify(value) ?? String(value);
  } catch {
    try {
      return Object.prototype.toString.call(value);
    } catch {
      return '(unrenderable)';
    }
  }
}

/**
 * Hand `payload` to `f`, which owns it from here. If f throws, Rust's unwind
 * drops the value f was given, so drop it before the throw carries on out —
 * otherwise it would belong to nobody and surface later as a leak.
 */
function callOwning<A, R>(f: (a: A) => R, payload: A): R {
  try {
    return f(payload);
  } catch (thrown) {
    dropOwned(payload);
    throw thrown;
  }
}

export class Result<T, E> extends Enum<ResultV<T, E>> {
  static Ok<T, E = never>(value: T): Result<T, E> {
    return new Result<T, E>('Ok', { _0: value });
  }

  static Err<T = never, E = unknown>(error: E): Result<T, E> {
    return new Result<T, E>('Err', { _0: error });
  }

  // ── &self in Rust: these borrow, so they read the Result and leave it whole.

  isOk(): boolean {
    return this.type === 'Ok';
  }

  isErr(): boolean {
    return this.type === 'Err';
  }

  // ── self in Rust: these consume the Result. Each takes the payload out and
  //    marks this one moved, so everything below owns a value this no longer
  //    does, and this Result can never be read or dropped again.

  /** Move the payload out. After this the Result is gone. */
  #consume(): { ok: boolean; payload: any } {
    const ok = this.type === 'Ok';
    const payload = (this.value as any)._0;
    this.markMoved();
    return { ok, payload };
  }

  unwrap(): T {
    const { ok, payload } = this.#consume();
    if (ok) return payload as T;
    const message = `called unwrap() on Err: ${describe(payload)}`;
    dropOwned(payload); // Rust panics here and the unwind drops the payload
    throw new Error(message);
  }

  unwrapErr(): E {
    const { ok, payload } = this.#consume();
    if (!ok) return payload as E;
    const message = `called unwrapErr() on Ok: ${describe(payload)}`;
    dropOwned(payload);
    throw new Error(message);
  }

  expect(message: string): T {
    const { ok, payload } = this.#consume();
    if (ok) return payload as T;
    const detail = `${message}: ${describe(payload)}`;
    dropOwned(payload);
    throw new Error(detail);
  }

  expectErr(message: string): E {
    const { ok, payload } = this.#consume();
    if (!ok) return payload as E;
    const detail = `${message}: ${describe(payload)}`;
    dropOwned(payload);
    throw new Error(detail);
  }

  unwrapOr(defaultValue: T): T {
    const { ok, payload } = this.#consume();
    if (ok) {
      dropOwned(defaultValue); // the default was not needed, and nobody else owns it
      return payload as T;
    }
    dropOwned(payload); // the error is discarded
    return defaultValue;
  }

  unwrapOrElse(f: (err: E) => T): T {
    const { ok, payload } = this.#consume();
    if (ok) return payload as T;
    return callOwning(f, payload as E); // the error moves into f
  }

  map<U>(f: (value: T) => U): Result<U, E> {
    const { ok, payload } = this.#consume();
    if (!ok) return Result.Err(payload as E); // the error moves into the new Result
    return Result.Ok(callOwning(f, payload as T));
  }

  mapErr<F>(f: (err: E) => F): Result<T, F> {
    const { ok, payload } = this.#consume();
    if (ok) return Result.Ok(payload as T); // the value moves into the new Result
    return Result.Err(callOwning(f, payload as E));
  }

  andThen<U>(f: (value: T) => Result<U, E>): Result<U, E> {
    const { ok, payload } = this.#consume();
    if (!ok) return Result.Err(payload as E);
    return callOwning(f, payload as T);
  }

  orElse<F>(f: (err: E) => Result<T, F>): Result<T, F> {
    const { ok, payload } = this.#consume();
    if (ok) return Result.Ok(payload as T);
    return callOwning(f, payload as E);
  }

  /** Rust's `ok()`: Result<T, E> -> Option<T>, which this port spells `T | null`. */
  ok(): T | null {
    const { ok, payload } = this.#consume();
    if (ok) return payload as T;
    dropOwned(payload);
    return null;
  }

  /** Rust's `err()`: Result<T, E> -> Option<E>. */
  err(): E | null {
    const { ok, payload } = this.#consume();
    if (!ok) return payload as E;
    dropOwned(payload);
    return null;
  }
}

/// Equivalent of Rust's ? operator for use in functions that return Result.
/// Extracts the Ok value, or returns the Err early.
/// Note: this can only be used as a statement, not inline, because TS has no early-return operator.
/// The transpiler generates:
///   const _r = foo();
///   if (_r.isErr()) return _r;
///   const x = _r.unwrap();
/// unwrap() consumes _r, so the Ok path needs no drop. A form that inspects the
/// Ok without consuming it must drop _r itself — `else _r.drop()` — or the
/// Result it built is a leak.
