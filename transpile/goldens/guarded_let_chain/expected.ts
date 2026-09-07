// MIRRORS: ankurah/guarded_let_chain/src/input.rs
import { Struct, Drop, Result } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function fromAResult(r: Result<Token, bigint>, allow: boolean): bigint {
  {
    const _v = r;
    if (_v.isOk()) {
      const token = _v.unwrap();
      if (allow) {
        try {
          return token.n;
        } finally {
          token.drop();
        }
      } else {
        token.drop();
        return 7n;
      }
    } else {
    _v.drop();
    return 7n;
  }
  }
}

export function fromAnOption(o: Token | null, allow: boolean): bigint {
  {
    const _v = o;
    if (_v != null) {
      const token = _v;
      if (allow) {
        try {
          return token.n;
        } finally {
          token.drop();
        }
      } else {
        token.drop();
        return 7n;
      }
    } else {
    return 7n;
  }
  }
}

export function allowed(o: Token | null): bigint {
  {
    const _v = o;
    if (_v != null) {
      const token = _v;
      if (token.n > 0n) {
        try {
          return token.n;
        } finally {
          token.drop();
        }
      } else {
        token.drop();
        return 7n;
      }
    } else {
    return 7n;
  }
  }
}

