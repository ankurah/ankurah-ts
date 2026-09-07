// MIRRORS: ankurah/evaluated_before_the_move/src/input.rs
import { Struct, Result, checkedAdd } from '@ankurah/base';

export class Token extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Op extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Oops extends Struct {
}

export class Pair extends Struct {
  readonly op: Op;
  readonly items: number[];
  readonly n: number;

  constructor(op: Op, items: number[], n: number) {
    super();
    this.op = op;
    this.items = items;
    this.n = n;
  }
}

export function take2(t: Token, n: number): number {
  try {
    return checkedAdd(t.n, n, 'u32');
  } finally {
    t.drop();
  }
}

export function laterThrows(t: Token, o: number | null): number {
  let _moved0 = false;
  try {
    const _b1 = (o ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
    _moved0 = true;
    return take2(t, _b1);
  } finally {
    if (!_moved0) t.drop();
  }
}

export function fallible(fail: boolean): Result<number, Oops> {
  if (fail) {
    return Result.Err(new Oops());
  } else {
    return Result.Ok(7);
  }
}

export function fieldAfterAQuestion(op: Op, fail: boolean): Result<Pair, Oops> {
  let _moved0 = false;
  try {
    const _r1 = fallible(fail);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    const _b2 = _r1.unwrap();
    _moved0 = true;
    return Result.Ok(new Pair(op, [], _b2));
  } finally {
    if (!_moved0) op.drop();
  }
}

