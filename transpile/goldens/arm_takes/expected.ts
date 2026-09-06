// MIRRORS: ankurah/arm_takes/src/input.rs
import { Struct, Enum, Drop, dropOwned, dropUnbound, unsupported, checkedAdd } from '@ankurah/base';

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

export type NamedV = {
  V: { copy: number; held: Token };
  Empty: {};
};

export class Named extends Enum<NamedV> {
}

export type MaybeV = {
  Some: { _0: Token };
  None: {};
};

export class Maybe extends Enum<MaybeV> {

  protected override onDrop(): void {

  }
}

export type OuterV = {
  W: { _0: Maybe };
  Nothing: {};
};

export class Outer extends Enum<OuterV> {
}

export type HolderV = {
  Pair: { _0: [Token, Token] };
  Nothing: {};
};

export class Holder extends Enum<HolderV> {
}

export function partial(pair: [Token, Token]): number {
  {
    const a = pair[0];
    try {
      {
        const n = a.n;
        a.drop();
        return n;
      }
    } finally {
      dropOwned(pair[1]);
    }
  }
}

export function nothing(pair: [Token, Token]): number {
  try {
    {
      return 0;
    }
  } finally {
    dropOwned(pair);
  }
}

export function both(pair: [Token, Token]): number {
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

export function outOfOrder(v: Named): number {
  return v.intoMatch({
    V: (v) => {
      const held = v.held;
      try {
        let _moved0 = false;
        try {
          const n = held.n;
          _moved0 = true;
          held.drop();
          return n;
        } finally {
          if (!_moved0) held.drop();
        }
      } finally {
        dropUnbound(v, ['held']);
      }
    },
    Empty: () => 0,
  });
}

export function userSome(o: Outer): number {
  return o.intoMatch({
    W: (v) => {
      if (v._0.is('Some')) {
        dropUnbound(v, []);
        unsupported('this arm tests inside `_0` and takes a DROPPABLE name out of it, and the port cannot both take a name out of a payload member and release what is left of it');
      } else {
        try {
          return 0;
        } finally {
          dropUnbound(v, []);
        }
      }
    },
    Nothing: () => 0,
  });
}

export function member(h: Holder): number {
  return h.intoMatch({
    Pair: (v) => {
      const [a, ] = v._0;
      try {
        let _moved0 = false;
        try {
          const n = a.n;
          _moved0 = true;
          a.drop();
          return n;
        } finally {
          if (!_moved0) a.drop();
        }
      } finally {
        dropOwned(v._0[1]);
      }
    },
    Nothing: () => 0,
  });
}

export function nestedPair(pair: [[Token, Token], Token]): number {
  {
    const a = pair[0][0];
    const c = pair[1];
    try {
      try {
        try {
          return checkedAdd(a.n, c.n, 'u32');
        } finally {
          c.drop();
        }
      } finally {
        a.drop();
      }
    } finally {
      dropOwned(pair[0][1]);
    }
  }
}

export function threeDeep(deep: [[[Token, Token], Token], Token]): number {
  {
    const a = deep[0][0][0];
    const d = deep[1];
    try {
      try {
        try {
          return checkedAdd(a.n, d.n, 'u32');
        } finally {
          d.drop();
        }
      } finally {
        a.drop();
      }
    } finally {
      dropOwned(deep[0][0][1]);
      dropOwned(deep[0][1]);
    }
  }
}

export function namedWhole(pair: [[Token, Token], Token]): number {
  {
    const inner = pair[0];
    const c = pair[1];
    try {
      try {
        return checkedAdd(inner[0].n, c.n, 'u32');
      } finally {
        c.drop();
      }
    } finally {
      dropOwned(inner);
    }
  }
}

