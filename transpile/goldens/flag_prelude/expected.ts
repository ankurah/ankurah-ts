// MIRRORS: ankurah/flag_prelude/src/input.rs
import { Struct, Drop, Result, dropOwned, checkedAdd } from '@ankurah/base';

export class Token extends Drop {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  protected override onDrop(): void {

  }
}

export class Held extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Sink extends Struct {

  swallow(t: Token, n: number): number {
    try {
      return n;
    } finally {
      t.drop();
    }
  }
}

export function eat(t: Token, n: number): number {
  try {
    return n;
  } finally {
    t.drop();
  }
}

export function eatTwo(t: Token, u: Token, n: number): number {
  try {
    try {
      return n;
    } finally {
      u.drop();
    }
  } finally {
    t.drop();
  }
}

export function make(): Token {
  return new Token(9);
}

export function boom(fail: boolean): Held {
  if (fail) {
    throw new Error('boom');
  }
  return new Held(1);
}

export function fieldOfCall(c: Token, fail: boolean, skip: boolean): number {
  let _moved0 = false;
  try {
    if (skip) {
      return 0;
    }
    const _t1 = boom(fail);
    try {
      const _b2 = _t1.n;
      _moved0 = true;
      return eat(c, _b2);
    } finally {
      _t1.drop();
    }
  } finally {
    if (!_moved0) c.drop();
  }
}

export function indexOfCall(c: Token, xs: number[], fail: boolean, skip: boolean): number {
  let _moved0 = false;
  try {
    if (skip) {
      return 0;
    }
    const _b1 = xs[which(fail)];
    _moved0 = true;
    return eat(c, _b1);
  } finally {
    if (!_moved0) c.drop();
  }
}

export function which(fail: boolean): number {
  if (fail) {
    throw new Error('boom');
  }
  return 0;
}

export function throwingReceiver(sink: Sink | null, c: Token, skip: boolean): number {
  let _moved0 = false;
  let _moved1 = false;
  try {
    try {
      if (skip) {
        return 0;
      }
      const _t2 = (sink ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
      try {
        _moved0 = true;
        _moved1 = true;
        return _t2.swallow(c, 1);
      } finally {
        _t2.drop();
      }
    } finally {
      if (!_moved1) c.drop();
    }
  } finally {
    if (!_moved0) dropOwned(sink);
  }
}

export function insideABranch(c: Token, o: number | null, skip: boolean): number {
  let _moved0 = false;
  try {
    if (skip) {
      return 0;
    }
    if (o != null) {
      const n = o;
      const _b1 = checkedAdd(n, 1, 'u32');
      _moved0 = true;
      return eat(c, _b1);
    } else {
      return 0;
    }
  } finally {
    if (!_moved0) c.drop();
  }
}

export function twoLifts(c: Token, o: number | null, skip: boolean): number {
  let _moved0 = false;
  try {
    if (skip) {
      return 0;
    }
    let _moved2 = false;
    const _b1 = make();
    try {
      const _b3 = (o ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
      _moved2 = true;
      _moved0 = true;
      return eatTwo(c, _b1, _b3);
    } finally {
      if (!_moved2) dropOwned(_b1);
    }
  } finally {
    if (!_moved0) c.drop();
  }
}

export function throughTry(c: Token, fail: boolean, skip: boolean): Result<number, string> {
  let _moved0 = false;
  try {
    if (skip) {
      return Result.Ok(0);
    }
    _moved0 = true;
    const _r1 = give(c, fail);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    const n = _r1.unwrap();
    return Result.Ok(n);
  } finally {
    if (!_moved0) c.drop();
  }
}

export function give(t: Token, fail: boolean): Result<number, string> {
  try {
    if (fail) {
      return Result.Err('no');
    }
    return Result.Ok(t._0);
  } finally {
    t.drop();
  }
}

