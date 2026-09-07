// MIRRORS: ankurah/fallible_dictionary/src/input.rs
import { Struct, Result, unsupported, checkedAdd } from '@ankurah/base';

export class Sel extends Struct {
  readonly text: string;

  constructor(text: string) {
    super();
    this.text = text;
  }

  static tryFrom(text: string): Result<Sel, Bad> {
    const _v = text.length === 0;
    if (_v === true) {
      return Result.Err(new Bad());
    } else {
      return Result.Ok(new Sel(text));
    }
  }

  static from(count: bigint): Sel {
    return new Sel(count.toString());
  }
}

export class Bad extends Struct {
}

export function pick<V>(v: V, _convV: (value: V) => Result<Sel, unknown>): Sel | null {
  return _convV(v).ok();
}

export function parsed(): Sel | null {
  return pick('year >= 2020', (value: string) => Sel.tryFrom(value));
}

export function refused(): Sel | null {
  return pick('', (value: string) => Sel.tryFrom(value));
}

export function counted(): Sel | null {
  return pick(7n, (value: bigint) => Result.Ok(Sel.from(value)));
}

export function paired<A>(a: A, count: bigint, _convA: (value: A) => Result<Sel, unknown>): bigint {
  const _v = pick(a, _convA);
  if (_v != null) {
    const sel = _v;
    try {
      return checkedAdd(BigInt(sel.text.length), count, 'i64');
    } finally {
      sel.drop();
    }
  } else {
    return count;
  }
}

export function shifted(): bigint {
  return paired((() => {
    const text = '';
    return text;
  })(), 7n, unsupported('the conversion for `A` cannot be named here'));
}

