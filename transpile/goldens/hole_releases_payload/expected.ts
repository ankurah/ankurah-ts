// MIRRORS: ankurah/hole_releases_payload/src/input.rs
import { Struct, Enum, Result, dropUnbound, unsupported, checkedAdd, HashMap, HashSet, keyHash } from '@ankurah/base';

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
        const a = unsupported('the alternatives of this pattern bind their names in a form the translator cannot read back — each alternative has to bind the same names, one `const` apiece — so this branch is a hole') as any;
        const b = unsupported('the alternatives of this pattern bind their names in a form the translator cannot read back — each alternative has to bind the same names, one `const` apiece — so this branch is a hole') as any;
        try {
          let _moved0 = false;
          let _moved1 = false;
          try {
            try {
              const n = checkedAdd(a.n, b.n, 'u32');
              _moved0 = true;
              a.drop();
              _moved1 = true;
              b.drop();
              return n;
            } finally {
              if (!_moved1) b.drop();
            }
          } finally {
            if (!_moved0) a.drop();
          }
        } finally {
          dropUnbound(v, ['_0']);
        }
      } else {
        const rest = v._1;
        try {
          let _moved2 = false;
          try {
            const n = rest.n;
            _moved2 = true;
            rest.drop();
            return n;
          } finally {
            if (!_moved2) rest.drop();
          }
        } finally {
          dropUnbound(v, ['_1']);
        }
      }
    },
    Empty: () => 0,
  });
}

