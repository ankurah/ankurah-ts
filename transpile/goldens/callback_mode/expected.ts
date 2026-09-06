// MIRRORS: ankurah/callback_mode/src/input.rs
import { Struct, Drop, OwnedClosure, dropOwned, unsupported, checkedAdd, iterFind, iterFindOwned } from '@ankurah/base';

export class Token extends Drop {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  protected override onDrop(): void {

  }
}

export function findOwning(tokens: Token[], want: Token): Token | null {
  return iterFindOwned([...tokens], new OwnedClosure([want], (t: Token) => t._0 === want._0));
}

export function findBorrowing(tokens: Token[], want: Token): Token | null {
  let p = new OwnedClosure([want], (t: Token) => t._0 === want._0);
  const found = iterFindOwned([...tokens], p, 'borrow');
  p.drop();
  return found;
}

export function readBorrowing(tokens: Token[], want: Token): number {
  const p = new OwnedClosure([want], (t: Token) => t._0 === want._0);
  let hits = 0;
  if ((iterFind([...tokens], p, 'borrow') != null)) {
    hits = checkedAdd(hits, 1, 'i32');
  }
  if ((iterFind([...tokens], p, 'borrow') != null)) {
    hits = checkedAdd(hits, 1, 'i32');
  }
  p.drop();
  return hits;
}

export function throughByRef(tokens: Token[]): Token | null {
  let it = [...tokens];
  try {
    return unsupported('`find` consumes the elements it walks and leaves the rest in the iterator this receiver names; the port writes an iterator as the whole array, so after the call it cannot say which of its elements are still the caller\'s');
  } finally {
    dropOwned(it);
  }
}

export function borrowedThroughByRef(tokens: Token[]): Token | null {
  let it = [...tokens];
  return iterFind(it, (t) => t._0 > 0);
}

