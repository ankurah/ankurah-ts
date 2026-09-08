// MIRRORS: ankurah/a_throw_above_a_move/src/input.rs
import { Struct, Drop, Result, checkedAdd } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function take(t: Token): bigint {
  try {
    return t.n;
  } finally {
    t.drop();
  }
}

export function fallible(ok: boolean): Result<bigint, string> {
  if (ok === true) {
    return Result.Ok(1n);
  } else {
    return Result.Err('no');
  }
}

export function throwsAboveAMove(a: Token, b: Token, o: bigint | null): Token[] {
  let _moved0 = false;
  let _moved1 = false;
  try {
    try {
      const _n = (o ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
      _moved0 = true;
      _moved1 = true;
      return [a, b];
    } finally {
      if (!_moved1) b.drop();
    }
  } finally {
    if (!_moved0) a.drop();
  }
}

export function throwsAboveOneMove(a: Token, o: bigint | null): Token {
  let _moved0 = false;
  try {
    const _n = (o ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
    _moved0 = true;
    return a;
  } finally {
    if (!_moved0) a.drop();
  }
}

export function aThrowingElementBeforeTheMove(a: Token, b: Token, ok: boolean): Result<bigint[], string> {
  let _moved0 = false;
  let _moved1 = false;
  try {
    try {
      const _r2 = fallible(ok);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      _moved0 = true;
      _moved1 = true;
      return Result.Ok([_r2.unwrap(), checkedAdd(take(a), take(b), 'i64')]);
    } finally {
      if (!_moved1) b.drop();
    }
  } finally {
    if (!_moved0) a.drop();
  }
}

export function theBindingOfTheThrowingStatement(n: bigint): bigint {
  const t = new Token(n);
  return take(t);
}

