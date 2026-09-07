// MIRRORS: ankurah/refusal_owns/src/input.rs
import { Struct, Drop, Result, dropOwned, unsupported, checkedAdd } from '@ankurah/base';

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
  let _moved2 = false;
  try {
    const _r0 = pass(first);
    _moved2 = true;
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    try {
      const _r1 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
      const _pair = [_r0.unwrap(), _r1];
      return Result.Ok(_pair[0].n);
    } finally {
      dropOwned(_r0);
    }
  } finally {
    if (!_moved2) first.drop();
    dropOwned(rest);
  }
}

export function onlyRefused(rest: Token[]): Result<number, string> {
  try {
    const _r0 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
    const _v = _r0;
    return Result.Ok(0);
  } finally {
    dropOwned(rest);
  }
}

export function movedThenRefused(held: Token, rest: Token[]): Result<number, string> {
  try {
    const _r0 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
    const _v = [take(held), _r0];
    return Result.Ok(0);
  } finally {
    held.drop();
    dropOwned(rest);
  }
}

export function take(t: Token): number {
  try {
    return t.n;
  } finally {
    t.drop();
  }
}

export function take2(a: Token, b: Result<Token[], string>): number {
  try {
    return checkedAdd(a.n, b.map((v) => {
      try {
        return v.length;
      } finally {
        dropOwned(v);
      }
    }).unwrapOr(0), 'u32');
  } finally {
    a.drop();
  }
}

export function refusedInTheText(held: Token, rest: Token[]): number {
  try {
    try {
      const _b1 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
      try {
        const _v = take2(held, _b1);
        return _v;
      } finally {
        dropOwned(_b1);
      }
    } finally {
      dropOwned(rest);
    }
  } finally {
    held.drop();
  }
}

export function refusedInALoop(items: Token[][]): number {
  let total = 0;
  const _seq1 = items;
  let _at2 = 0;
  try {
    while (_at2 < _seq1.length) {
      const rest = _seq1[_at2++];
      try {
        const _v = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
        total = checkedAdd(total, 1, 'i32');
      } finally {
        dropOwned(rest);
      }
    }
  } finally {
    dropOwned(_seq1.slice(_at2));
  }
  return total;
}

export function count(xs: Token[]): Result<number, string> {
  try {
    return Result.Ok(xs.length);
  } finally {
    dropOwned(xs);
  }
}

export function vecHandedOverFirst(rest: Token[], more: Token[]): Result<number, string> {
  let _moved2 = false;
  try {
    const _r0 = count(rest);
    _moved2 = true;
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    try {
      const _r1 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
      const _pair = [_r0.unwrap(), _r1];
      return Result.Ok(0);
    } finally {
      dropOwned(_r0);
    }
  } finally {
    if (!_moved2) dropOwned(rest);
    dropOwned(more);
  }
}

export function vecNeverHandedOver(rest: Token[], more: Token[]): Result<number, string> {
  let _moved3 = false;
  try {
    const _r0 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
    const _r1 = count(rest);
    _moved3 = true;
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    try {
      const _pair = [_r0, _r1.unwrap()];
      return Result.Ok(0);
    } finally {
      dropOwned(_r1);
    }
  } finally {
    dropOwned(more);
    if (!_moved3) dropOwned(rest);
  }
}

