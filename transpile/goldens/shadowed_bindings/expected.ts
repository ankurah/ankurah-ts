// MIRRORS: ankurah/shadowed_bindings/src/input.rs
import { Struct, Drop, invokeRef, Invocable, dropOwned } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function apply(t: Token, f: Invocable<[Token], bigint>): bigint {
  try {
    return invokeRef(f, t);
  } finally {
    dropOwned(f);
  }
}

export function shadowMoved(token: Token): bigint {
  try {
    const token_1 = new Token(2n);
    token_1.drop();
    return 1n;
  } finally {
    token.drop();
  }
}

export function shadowReturned(t: Token): Token {
  try {
    const t_1 = new Token(9n);
    return t_1;
  } finally {
    t.drop();
  }
}

export function shadowInAClosure(): bigint {
  return apply(new Token(1n), (token) => {
    try {
      const token_1 = new Token(2n);
      token_1.drop();
      return 1n;
    } finally {
      token.drop();
    }
  });
}

export function shadowInAnArm(): bigint {
  let _moved1 = false;
  const _b0 = new Token(1n);
  try {
    const _b2 = new Token(2n);
    _moved1 = true;
    const pair = [_b0, _b2];
    {
      const a = pair[0];
      const b = pair[1];
      try {
        try {
          {
            const a_1 = new Token(3n);
            a_1.drop();
            return b.n;
          }
        } finally {
          b.drop();
        }
      } finally {
        a.drop();
      }
    }
  } finally {
    if (!_moved1) dropOwned(_b0);
  }
}

