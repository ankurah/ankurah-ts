// MIRRORS: ankurah/refusal_owns/src/input.rs
import { Struct, Drop, Result, dropOwned, unsupported } from '@ankurah/base';

export class Token extends Drop {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  static new(n: number): Token {
    return new Token(n);
  }

  protected override onDrop(): void {

  }
}

export function pass(t: Token): Result<Token, string> {
  return Result.Ok(t);
}

export function nested(first: Token, rest: Token[]): Result<number, string> {
  try {
    const _r0 = pass(first);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    try {
      const _r1 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
      const _pair = [_r0.unwrap(), _r1];
      return Result.Ok(_pair[0].n);
    } finally {
      if (_r0 != null && !(_r0 as any).isMoved && !(_r0 as any).isDropped) dropOwned(_r0);
    }
  } finally {
    if (first != null && !(first as any).isMoved && !(first as any).isDropped) dropOwned(first);
    if (rest != null && !(rest as any).isMoved && !(rest as any).isDropped) dropOwned(rest);
  }
}

export function onlyRefused(rest: Token[]): Result<number, string> {
  try {
    const _r0 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
    const _v = _r0;
    return Result.Ok(0);
  } finally {
    if (rest != null && !(rest as any).isMoved && !(rest as any).isDropped) dropOwned(rest);
  }
}

export function movedThenRefused(held: Token, rest: Token[]): Result<number, string> {
  try {
    const _r0 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
    const _v = [take(held), _r0];
    return Result.Ok(0);
  } finally {
    if (held != null && !(held as any).isMoved && !(held as any).isDropped) dropOwned(held);
    if (rest != null && !(rest as any).isMoved && !(rest as any).isDropped) dropOwned(rest);
  }
}

export function take(t: Token): number {
  try {
    return t.n;
  } finally {
    t.drop();
  }
}

