// MIRRORS: ankurah/owned_adaptors/src/input.rs
import { Struct, Drop, iterFind, iterFirst, iterMaxByKey, iterFindOwned, iterLastOwned, iterMaxByKeyOwned, iterFirstOwned, filterOwned, skipOwned, takeOwned, stepByOwned } from '@ankurah/base';

export class Token extends Drop {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  protected override onDrop(): void {

  }
}

export class Key extends Drop {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  protected override onDrop(): void {

  }

  equals(other: Key): boolean {
    if (this._0 !== other._0) return false;
    return true;
  }

  compareTo(other: Key): number {
    let c = this._0 < other._0 ? -1 : this._0 > other._0 ? 1 : 0;
    if (c !== 0) return c;
    return 0;
  }
}

export function firstOver(tokens: Token[], want: number): Token | null {
  return iterFindOwned(filterOwned([...tokens], (t) => t._0 > 0), (t) => t._0 === want);
}

export function everyOther(tokens: Token[]): Token | null {
  return iterLastOwned(stepByOwned([...tokens], 2));
}

export function middle(tokens: Token[]): Token | null {
  return iterLastOwned(takeOwned(skipOwned([...tokens], 1), 1));
}

export function borrowedFilter(tokens: Token[], want: number): Token | null {
  return iterFind([...tokens].filter((t) => t._0 > 0), (t) => t._0 === want);
}

export function widest(tokens: Token[]): Token | null {
  return iterMaxByKey([...tokens], (t) => new Key(t._0));
}

export function widestOwned(tokens: Token[]): Token | null {
  return iterMaxByKeyOwned([...tokens], (t) => new Key(t._0));
}

export function firstOwned(tokens: Token[]): Token | null {
  return iterFirstOwned([...tokens]);
}

export function firstBorrowed(tokens: Token[]): Token | null {
  return iterFirst([...tokens]);
}

