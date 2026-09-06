// MIRRORS: ankurah/opaque_iterator/src/input.rs
import { Struct, Result, dropOwned, checkedAdd, SeqCursor } from '@ankurah/base';

export class Token extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Refused extends Struct {
}

export function takeSome<I extends Iterable<Token>>(values: SeqCursor<Token>, wanted: number): Result<number, Refused> {
  let total = 0;
  let taken = 0;
  while (taken < wanted) {
    const _v = values.next();
    if (_v != null) {
      const token = _v;
      try {
        total = checkedAdd(total, token.n, 'i32');
        taken = checkedAdd(taken, 1, 'i32');
      } finally {
        token.drop();
      }
    } else {
      return Result.Err(new Refused())
    }
  }
  return Result.Ok(total);
}

export function sumFirst<I extends Iterable<Token>>(values: I, wanted: number): Result<number, Refused> {
  let walk = new SeqCursor([...values]);
  try {
    const _r0 = takeSome(walk, wanted);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const total = _r0.unwrap();
    let _c2;
    const _t1 = walk.next();
    try {
      _c2 = (_t1 != null);
    } finally {
      dropOwned(_t1);
    }
    if (_c2) {
      return Result.Err(new Refused());
    }
    return Result.Ok(total);
  } finally {
    walk.drop();
  }
}

export function restOf<I extends Iterable<Token>>(values: I, skip: number): Result<Token[], Refused> {
  let _moved0 = false;
  let walk = new SeqCursor([...values]);
  try {
    const _r1 = takeSome(walk, skip);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    _r1.drop();
    let _moved2 = false;
    let kept = [];
    try {
      _moved0 = true;
      for (const token of walk.takeRest()) {
        kept.push(token);
      }
      _moved2 = true;
      return Result.Ok(kept);
    } finally {
      if (!_moved2) dropOwned(kept);
    }
  } finally {
    if (!_moved0) walk.drop();
  }
}

