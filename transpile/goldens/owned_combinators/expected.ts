// MIRRORS: ankurah/owned_combinators/src/input.rs
import { Struct, Result, OwnedClosure, invoke, dropOwned, checkedAdd } from '@ankurah/base';

export class Token extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  static new(n: number): Token {
    return new Token(n);
  }
}

export function source(n: number): number | null {
  if (n === 0) {
    return null;
  } else {
    return n;
  }
}

export function eager(): number {
  return 7;
}

export function nested(): number {
  const _m0 = source(1);
  const _m1 = source(2);
  const _m2 = eager();
  const _m3 = (_m1 != null ? ((v) => v)(_m1!) : _m2);
  return (_m0 != null ? ((v) => v)(_m0!) : _m3);
}

export function mapCapture(value: number | null, token: Token): number | null {
  const _m0 = new OwnedClosure([token], (n: number) => {
    const m = token.n;
    token.drop();
    return checkedAdd(n, m, 'u32');
  }, undefined, true);
  return (value != null ? invoke(_m0, value!) : (dropOwned(_m0), null));
}

export function andThenCapture(value: number | null, token: Token): number | null {
  const _m0 = new OwnedClosure([token], (n: number) => {
    const m = token.n;
    token.drop();
    return checkedAdd(n, m, 'u32');
  }, undefined, true);
  return (value != null ? invoke(_m0, value!) : (dropOwned(_m0), null));
}

export function filterCapture(value: number | null, token: Token): number | null {
  return (value != null && ((n) => n > token.n)(value!) ? value : null);
}

export function filterOwned(value: number | null, token: Token): number | null {
  const _m0 = new OwnedClosure([token], (n: number) => {
    const m = token.n;
    token.drop();
    return n > m;
  }, undefined, true);
  return (value != null ? (invoke(_m0, value!) ? value : null) : (dropOwned(_m0), null));
}

export function isSomeAndOwned(value: number | null, token: Token): boolean {
  const _m0 = new OwnedClosure([token], (n: number) => {
    const m = token.n;
    token.drop();
    return n > m;
  }, undefined, true);
  return (value != null ? invoke(_m0, value!) : (dropOwned(_m0), false));
}

export function mapOrCapture(value: number | null, token: Token): number {
  const _m0 = new OwnedClosure([token], (n: number) => {
    const m = token.n;
    token.drop();
    return checkedAdd(n, m, 'u32');
  }, undefined, true);
  return (value != null ? invoke(_m0, value!) : (dropOwned(_m0), 0));
}

export function mapOrElseCapture(value: number | null, token: Token, other: Token): number {
  const _m0 = new OwnedClosure([other], () => other.n);
  const _m1 = new OwnedClosure([token], (n: number) => {
    const m = token.n;
    token.drop();
    return checkedAdd(n, m, 'u32');
  }, undefined, true);
  return (value != null ? (() => { try { return invoke(_m1, value!); } finally { dropOwned(_m0); } })() : (() => { try { return invoke(_m0); } finally { dropOwned(_m1); } })());
}

export function okOrElseCapture(value: number | null, token: Token): Result<number, number> {
  const _m0 = new OwnedClosure([token], () => {
    const m = token.n;
    token.drop();
    return m;
  }, undefined, true);
  return (value != null ? (dropOwned(_m0), Result.Ok(value!)) : Result.Err(invoke(_m0)));
}

export function namedClosure(value: number | null, token: Token): number | null {
  const f = new OwnedClosure([token], (n: number) => {
    const m = token.n;
    token.drop();
    return checkedAdd(n, m, 'u32');
  }, undefined, true);
  return (value != null ? invoke(f, value!) : (dropOwned(f), null));
}

