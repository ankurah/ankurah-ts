// MIRRORS: ankurah/nested_exit/src/input.rs
import { Struct, Enum, Result, BorrowMut } from '@ankurah/base';

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
  Good: {};
  Bad: {};
};

export class Inner extends Enum<InnerV> {
}

export type OuterV = {
  One: { _0: Token };
  Two: {};
};

export class Outer extends Enum<OuterV> {
}

export function run(outer: Outer, inner: Inner, out: BorrowMut<string>): Result<number, string> {
  const _m2 = outer.intoMatch<any>({
    One: (v) => {
      const token = v._0;
      let _moved0 = false;
      try {
        const _m1 = inner.match<any>({
          Good: () => {
            out.value += 'g';
          },
          Bad: () => {
            return { $jump: 'return', $value: Result.Err('bad') };
          },
        });
        if ((_m1 as any)?.$jump === 'return') return _m1;
        out.value += '1';
        const n = token.n;
        _moved0 = true;
        token.drop();
        return { $jump: 'return', $value: Result.Ok(n) };
      } finally {
        if (!_moved0) token.drop();
      }
    },
    Two: () => {
      out.value += '2';
    },
  });
  if ((_m2 as any)?.$jump === 'return') return (_m2 as any).$value;
  return Result.Ok(0);
}

