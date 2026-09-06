// MIRRORS: ankurah/owned_terminals/src/input.rs
import { Struct, Drop, checkedRem, iterFind, iterLast, iterPositionOwned, iterFindOwned, iterLastOwned, iterMinByOwned, iterMaxByKeyOwned, iterReduceOwned } from '@ankurah/base';

export class Token extends Drop {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  protected override onDrop(): void {

  }
}

export function positionOf(tokens: Token[], want: number): number | null {
  return iterPositionOwned([...tokens], (token) => {
    const hit = token._0 === want;
    token.drop();
    return hit;
  });
}

export function findOne(tokens: Token[], want: number): Token | null {
  return iterFindOwned([...tokens], (t) => t._0 === want);
}

export function biggest(tokens: Token[]): Token | null {
  return iterMaxByKeyOwned([...tokens], (t) => t._0);
}

export function smallest(tokens: Token[]): Token | null {
  return iterMinByOwned([...tokens], (a, b) => (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a._0, b._0));
}

export function firstKept(tokens: Token[]): Token | null {
  return iterReduceOwned([...tokens], (a, b) => {
    b.drop();
    return a;
  });
}

export function lastOf(tokens: Token[]): Token | null {
  return iterLastOwned([...tokens]);
}

export function peekLast(tokens: Token[]): Token | null {
  return iterLast(tokens);
}

export function borrowedFind(tokens: Token[], want: number): Token | null {
  return iterFind([...tokens], (t) => t._0 === want);
}

export function firstEven(ns: number[]): number | null {
  return iterFind([...ns], (n) => checkedRem(n, 2, 'u32') === 0);
}

