// MIRRORS: ankurah/hole_releases_payload/src/input.rs
import { Struct, Enum, Result, dropUnbound, unsupported, HashMap, keyHash } from '@ankurah/base';

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

export class Name extends Struct {
  readonly text: string;

  constructor(text: string) {
    super();
    this.text = text;
  }

  equals(other: Name): boolean {
    if (this.text !== other.text) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.text)].map((p) => p.length + ':' + p).join('');
  }
}

export class Counts extends Struct {
  readonly m: HashMap<Name, Result<number, number>>;

  constructor(m: HashMap<Name, Result<number, number>>) {
    super();
    this.m = m;
  }

  finish(k: Name): number {
    try {
      unsupported('`or_default()` needs the value type\'s default, and the port writes `Result<u32, u32>` as Result([Prim(U32), Prim(U32)]), which has no default value');
      return 0;
    } finally {
      k.drop();
    }
  }
}

export type InnerV = {
  A: { _0: [Token, Token] };
  B: { _0: [Token, Token] };
};

export class Inner extends Enum<InnerV> {
}

export type WrapV = {
  Held: { _0: Inner; _1: Token };
  Empty: {};
};

export class Wrap extends Enum<WrapV> {
}

export function pick(w: Wrap): number {
  return w.intoMatch({
    Held: (v) => {
      if ((v._0.is('A')) || (v._0.is('B'))) {
        dropUnbound(v, []);
        unsupported('this arm tests inside `_0` and takes a DROPPABLE name out of it, and the port cannot both take a name out of a payload member and release what is left of it');
      } else {
        const rest = v._1;
        try {
          let _moved0 = false;
          try {
            const n = rest.n;
            _moved0 = true;
            rest.drop();
            return n;
          } finally {
            if (!_moved0) rest.drop();
          }
        } finally {
          dropUnbound(v, ['_1']);
        }
      }
    },
    Empty: () => 0,
  });
}

