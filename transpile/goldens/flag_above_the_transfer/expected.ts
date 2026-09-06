// MIRRORS: ankurah/flag_above_the_transfer/src/input.rs
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

export function gate(open: boolean): Result<number, Refused> {
  if (open) {
    return Result.Ok(1);
  } else {
    return Result.Err(new Refused());
  }
}

export function build(explode: boolean): Token {
  if (explode) {
    throw new Error('build exploded');
  }
  return new Token(4);
}

export function eat(a: Token, b: Token): Result<number, Refused> {
  try {
    try {
      return Result.Ok(checkedAdd(a.n, b.n, 'u32'));
    } finally {
      b.drop();
    }
  } finally {
    a.drop();
  }
}

export function consume(t: Token, fail: boolean): Result<number, Refused> {
  try {
    if (fail) {
      return Result.Err(new Refused());
    }
    return Result.Ok(t.n);
  } finally {
    t.drop();
  }
}

export function lifted(explode: boolean): Result<number, Refused> {
  let _moved0 = false;
  const held = new Token(1);
  try {
    const _r1 = gate(true);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    const opened = _r1.unwrap();
    const _b2 = build(explode);
    _moved0 = true;
    const _r3 = eat(_b2, held);
    if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
    const total = _r3.unwrap();
    return Result.Ok(checkedAdd(total, opened, 'u32'));
  } finally {
    if (!_moved0) held.drop();
  }
}

export function twoTransfers(fail: boolean): Result<number, Refused> {
  let _moved0 = false;
  const first = new Token(1);
  try {
    let _moved1 = false;
    const second = new Token(2);
    try {
      const _r2 = gate(true);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      const opened = _r2.unwrap();
      _moved0 = true;
      const _r3 = consume(first, fail);
      if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
      try {
        _moved1 = true;
        const _r4 = consume(second, false);
        if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
        try {
          const total = checkedAdd(_r3.unwrap(), _r4.unwrap(), 'u32');
          return Result.Ok(checkedAdd(total, opened, 'u32'));
        } finally {
          if (_r4 != null && !(_r4 as any).isMoved && !(_r4 as any).isDropped) dropOwned(_r4);
        }
      } finally {
        if (_r3 != null && !(_r3 as any).isMoved && !(_r3 as any).isDropped) dropOwned(_r3);
      }
    } finally {
      if (!_moved1) second.drop();
    }
  } finally {
    if (!_moved0) first.drop();
  }
}

export function oneTransfer(fail: boolean): Result<number, Refused> {
  let _moved0 = false;
  const held = new Token(3);
  try {
    const _r1 = gate(true);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    const opened = _r1.unwrap();
    _moved0 = true;
    const _r2 = consume(held, fail);
    if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
    const total = _r2.unwrap();
    return Result.Ok(checkedAdd(total, opened, 'u32'));
  } finally {
    if (!_moved0) held.drop();
  }
}

