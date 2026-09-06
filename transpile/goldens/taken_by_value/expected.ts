// MIRRORS: ankurah/taken_by_value/src/input.rs
import { Struct, Drop, dropOwned, checkedAdd, iterFind, iterPositionOwned, iterReduceOwned } from '@ankurah/base';

export class Token extends Drop {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  protected override onDrop(): void {

  }
}

export class Holder extends Struct {
  readonly item: Token;
  readonly tag: number;

  constructor(item: Token, tag: number) {
    super();
    this.item = item;
    this.tag = tag;
  }
}

export function positionOf(tokens: Token[], want: number): number | null {
  return iterPositionOwned([...tokens], (token) => {
    try {
      return token._0 === want;
    } finally {
      token.drop();
    }
  });
}

export function positionOrFail(tokens: Token[], bad: number): number | null {
  return iterPositionOwned([...tokens], (token) => {
    try {
      if (token._0 === bad) {
        throw new Error('bad token');
      }
      return false;
    } finally {
      token.drop();
    }
  });
}

export function firstKept(tokens: Token[]): Token | null {
  return iterReduceOwned([...tokens], (a, b) => {
    b.drop();
    return a;
  });
}

export function items(holders: Holder[]): Token[] {
  return [...holders].map((holder) => {
    try {
      return holder.takeField('item');
    } finally {
      holder.drop();
    }
  });
}

export function findBorrowed(tokens: Token[], want: number): Token | null {
  return iterFind([...tokens], (t) => t._0 === want);
}

export function total(a: Token, b: Token): number {
  const _v = [a, b];
  {
    const x = _v[0];
    const y = _v[1];
    try {
      try {
        return checkedAdd(x._0, y._0, 'u32');
      } finally {
        y.drop();
      }
    } finally {
      x.drop();
    }
  }
}

export function keepFirst(a: Token | null, b: Token | null): Token | null {
  const _v = [a, b];
  if ((_v[0] != null)) {
    const x = _v[0];
    try {
      return x;
    } finally {
      dropOwned(_v[1]);
    }
  } else {
    const other = _v[1];
    return other;
  }
}

