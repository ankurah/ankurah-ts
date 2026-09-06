// MIRRORS: ankurah/payload_taking/src/input.rs
import { Struct, Enum, Result, dropUnbound, unsupported, checkedAdd } from '@ankurah/base';

export class Token extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  static new(n: number): Token {
    return new Token(n);
  }
}

export type InnerV = {
  X: { _0: Token };
  Y: { _0: Token };
};

export class Inner extends Enum<InnerV> {
}

export type OuterV = {
  W: { _0: Inner };
  Z: {};
};

export class Outer extends Enum<OuterV> {
}

export type CountV = {
  Small: { _0: number };
  Large: { _0: number };
};

export class Count extends Enum<CountV> {
}

export type HolderV = {
  Held: { _0: Count };
  Empty: {};
};

export class Holder extends Enum<HolderV> {
}

export function inside(o: Outer): number {
  return o.intoMatch({
    W: (v) => {
      if (v._0.is('X')) {
        dropUnbound(v, []);
        unsupported('this arm tests inside `_0` and takes a DROPPABLE name out of it, and the port cannot both take a name out of a payload member and release what is left of it');
      } else {
        try {
          return 1;
        } finally {
          dropUnbound(v, []);
        }
      }
    },
    Z: () => 0,
  });
}

export function either(o: Outer): number {
  return o.intoMatch({
    W: (v) => {
      dropUnbound(v, []);
      unsupported('this arm tests inside `_0` and takes a DROPPABLE name out of it, and the port cannot both take a name out of a payload member and release what is left of it');
    },
    Z: () => 0,
  });
}

export function counted(h: Holder): number {
  return h.intoMatch({
    Held: (v) => {
      const { _0: n } = v._0.value;
      try {
        return n;
      } finally {
        dropUnbound(v, []);
      }
    },
    Empty: () => 0,
  });
}

export function both(left: Result<Token, number>, right: Result<Token, number>): number {
  const _v = [left, right];
  if ((_v[0].isOk()) && (_v[1].isOk())) {
    const l = _v[0].okRef();
    const r = _v[1].okRef();
    return checkedAdd(l.n, r.n, 'u32');
  } else {
    return 0;
  }
}

export function consumed(pair: [Token, Token]): number {
  {
    const a = pair[0];
    const b = pair[1];
    {
      const n = checkedAdd(a.n, b.n, 'u32');
      a.drop();
      b.drop();
      return n;
    }
  }
}

