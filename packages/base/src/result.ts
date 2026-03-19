// TS-ONLY: Rust's Result<T, E> type — an Enum with Ok and Err variants.
//
// Usage mirrors Rust:
//   Result.Ok(value)     — construct an Ok variant
//   Result.Err(error)    — construct an Err variant
//   result.isOk()        — check if Ok
//   result.isErr()       — check if Err
//   result.unwrap()      — extract Ok value or throw
//   result.unwrapErr()   — extract Err value or throw
//   result.match({ Ok: v => ..., Err: e => ... })

import { Enum } from './enum.ts';

type ResultV<T, E> = {
  Ok: { _0: T };
  Err: { _0: E };
};

export class Result<T, E> extends Enum<ResultV<T, E>> {
  static Ok<T, E = never>(value: T): Result<T, E> {
    return new Result<T, E>('Ok', { _0: value });
  }

  static Err<T = never, E = unknown>(error: E): Result<T, E> {
    return new Result<T, E>('Err', { _0: error });
  }

  isOk(): boolean {
    return this.type === 'Ok';
  }

  isErr(): boolean {
    return this.type === 'Err';
  }

  unwrap(): T {
    if (this.type === 'Ok') {
      return (this.value as ResultV<T, E>['Ok'])._0;
    }
    throw new Error(`called unwrap() on Err: ${(this.value as ResultV<T, E>['Err'])._0}`);
  }

  unwrapErr(): E {
    if (this.type === 'Err') {
      return (this.value as ResultV<T, E>['Err'])._0;
    }
    throw new Error('called unwrapErr() on Ok');
  }

  unwrapOr(defaultValue: T): T {
    if (this.type === 'Ok') {
      return (this.value as ResultV<T, E>['Ok'])._0;
    }
    return defaultValue;
  }

  unwrapOrElse(f: (err: E) => T): T {
    if (this.type === 'Ok') {
      return (this.value as ResultV<T, E>['Ok'])._0;
    }
    return f((this.value as ResultV<T, E>['Err'])._0);
  }

  map<U>(f: (value: T) => U): Result<U, E> {
    if (this.type === 'Ok') {
      return Result.Ok(f((this.value as ResultV<T, E>['Ok'])._0));
    }
    return Result.Err((this.value as ResultV<T, E>['Err'])._0);
  }

  mapErr<F>(f: (err: E) => F): Result<T, F> {
    if (this.type === 'Err') {
      return Result.Err(f((this.value as ResultV<T, E>['Err'])._0));
    }
    return Result.Ok((this.value as ResultV<T, E>['Ok'])._0);
  }

  /// Rust's ? operator equivalent — returns the Ok value, or propagates the Err
  /// Usage: const x = tryResult(foo()); // in a function that returns Result
  /// This is a standalone function, not a method, because ? affects control flow.
}

/// Equivalent of Rust's ? operator for use in functions that return Result.
/// Extracts the Ok value, or returns the Err early.
/// Note: this can only be used as a statement, not inline, because TS has no early-return operator.
/// The transpiler generates: const _r = foo(); if (_r.isErr()) return _r; const x = _r.unwrap();
