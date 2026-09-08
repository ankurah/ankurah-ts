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
  let _moved0 = false;
  let p = new OwnedClosure([want], (t: Token) => t._0 === want._0);
  try {
    let _moved1 = false;
    const found = iterFindOwned([...tokens], p, 'borrow');
    try {
      _moved0 = true;
      p.drop();
      _moved1 = true;
      return found;
    } finally {
      if (!_moved1) dropOwned(found);
    }
  } finally {
    if (!_moved0) p.drop();
  }
}

export function readBorrowing(tokens: Token[], want: Token): number {
  let _moved0 = false;
  const p = new OwnedClosure([want], (t: Token) => t._0 === want._0);
  try {
    let hits = 0;
    if ((iterFind([...tokens], p, 'borrow') != null)) {
      hits = checkedAdd(hits, 1, 'i32');
    }
    if ((iterFind([...tokens], p, 'borrow') != null)) {
      hits = checkedAdd(hits, 1, 'i32');
    }
    _moved0 = true;
    p.drop();
    return hits;
  } finally {
    if (!_moved0) p.drop();
  }
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

