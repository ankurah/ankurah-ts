// MIRRORS: ankurah/leaving_early/src/input.rs
import { Struct, Drop, Result, dropOwned, checkedAdd, countOwned, filterOwned } from '@ankurah/base';

export class Token extends Drop {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function passr(t: Token): Result<Token, number> {
  return Result.Ok(t);
}

export function eat(v: Token[]): number {
  try {
    return v.length;
  } finally {
    dropOwned(v);
  }
}

export function maybeTokens(flag: boolean): Token[] | null {
  if (flag === true) {
    return [new Token(1), new Token(2)];
  } else {
    return null;
  }
}

export async function awaited(t: Token, f: Promise<number>): Promise<Result<[number, Token], number>> {
  let _moved0 = false;
  try {
    _moved0 = true;
    const _r1 = passr(t);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    try {
      return Result.Ok([await f, _r1.unwrap()]);
    } finally {
      if (_r1 != null && !(_r1 as any).isMoved && !(_r1 as any).isDropped) dropOwned(_r1);
    }
  } finally {
    if (!_moved0) t.drop();
  }
}

export function twoPayloads(a: boolean, b: boolean): number | null {
  let _r0_kept = false;
  const _r0 = maybeTokens(a);
  if (_r0 == null) return null;
  try {
    let _r1_kept = false;
    const _r1 = maybeTokens(b);
    if (_r1 == null) return null;
    try {
      _r0_kept = true;
      _r1_kept = true;
      return checkedAdd(eat(_r0), eat(_r1), 'u32');
    } finally {
      if (!_r1_kept) dropOwned(_r1);
    }
  } finally {
    if (!_r0_kept) dropOwned(_r0);
  }
}

export function counted(tokens: Token[]): number {
  return countOwned(filterOwned([...tokens], (t) => t.n > 1));
}

