// MIRRORS: ankurah/hole_releases_payload/src/input.rs
import { Struct, Enum, dropUnbound, unsupported, checkedAdd } from '@ankurah/base';

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
          let _moved2 = false;
          let _moved3 = false;
          try {
            try {
              try {
                try {
                  const n = checkedAdd(a.n, b.n, 'u32');
                  _moved3 = true;
                  a.drop();
                  _moved2 = true;
                  b.drop();
                  return n;
                } finally {
                  if (!_moved3) a.drop();
                }
              } finally {
                if (!_moved2) b.drop();
              }
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
          let _moved4 = false;
          try {
            const n = rest.n;
            _moved4 = true;
            rest.drop();
            return n;
          } finally {
            if (!_moved4) rest.drop();
          }
        } finally {
          dropUnbound(v, ['_1']);
        }
      }
    },
    Empty: () => 0,
  });
}

