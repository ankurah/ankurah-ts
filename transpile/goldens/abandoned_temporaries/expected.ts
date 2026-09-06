// MIRRORS: ankurah/abandoned_temporaries/src/input.rs
import { Struct, Result, dropOwned, checkedAdd } from '@ankurah/base';

export class Token extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Refused extends Struct {
}

export function take(n: number): Result<Token, Refused> {
  if (n === 0) {
    return Result.Err(new Refused());
  }
  return Result.Ok(new Token(n));
}

export function both(a: number, b: number): Result<[Token, Token], Refused> {
  const _r0 = take(a);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  try {
    const _r1 = take(b);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    try {
      return Result.Ok([_r0.unwrap(), _r1.unwrap()]);
    } finally {
      if (_r1 != null && !(_r1 as any).isMoved && !(_r1 as any).isDropped) dropOwned(_r1);
    }
  } finally {
    if (_r0 != null && !(_r0 as any).isMoved && !(_r0 as any).isDropped) dropOwned(_r0);
  }
}

export function three(a: number, b: number, c: number): Result<number, Refused> {
  const _r0 = take(a);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const _t1 = _r0.unwrap();
  try {
    const _r2 = take(b);
    if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
    const _t3 = _r2.unwrap();
    try {
      const _r4 = take(c);
      if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
      const _t5 = _r4.unwrap();
      try {
        const sum = checkedAdd(checkedAdd(_t1.n, _t3.n, 'u32'), _t5.n, 'u32');
        return Result.Ok(sum);
      } finally {
        _t5.drop();
      }
    } finally {
      _t3.drop();
    }
  } finally {
    _t1.drop();
  }
}

export function bothOrPanic(a: number, b: number): Result<[Token, Token], Refused> {
  const _r0 = take(a);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  try {
    const _r1 = exploding(b);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    try {
      return Result.Ok([_r0.unwrap(), _r1.unwrap()]);
    } finally {
      if (_r1 != null && !(_r1 as any).isMoved && !(_r1 as any).isDropped) dropOwned(_r1);
    }
  } finally {
    if (_r0 != null && !(_r0 as any).isMoved && !(_r0 as any).isDropped) dropOwned(_r0);
  }
}

export function exploding(n: number): Result<Token, Refused> {
  if (n === 99) {
    throw new Error('exploding was asked for 99');
  }
  return take(n);
}

export function onlyOne(a: number): Result<number, Refused> {
  const _r0 = take(a);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const t = _r0.unwrap();
  try {
    return Result.Ok(t.n);
  } finally {
    t.drop();
  }
}

