// MIRRORS: ankurah/consuming_alternatives/src/input.rs
import { Struct, Enum, Drop, Result, dropUnbound, SeqCursor } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export type ShapeV = {
  One: { _0: Token };
  Two: { _0: Token; _1: bigint };
  Nothing: {};
};

export class Shape extends Enum<ShapeV> {
}

export type DuoV = {
  A: { _0: Token };
  B: {};
};

export class Duo extends Enum<DuoV> {
}

export type OutcomeV = {
  Ok: { _0: Token };
  Other: {};
};

export class Outcome extends Enum<OutcomeV> {
}

export function either(s: Shape): bigint {
  return s.intoMatch({
    One: (v) => {
      const t = v._0;
      try {
        return t.n;
      } finally {
        t.drop();
      }
    },
    Two: (v) => {
      const t = v._0;
      try {
        try {
          return t.n;
        } finally {
          t.drop();
        }
      } finally {
        dropUnbound(v, ['_0']);
      }
    },
    Nothing: () => {
      return 0n;
    },
  });
}

export function nestedStruct(d: Duo): bigint {
  return d.intoMatch({
    A: (v) => {
      const { n } = v._0;
      try {
        return n;
      } finally {
        dropUnbound(v, []);
      }
    },
    B: () => {
      return 0n;
    },
  });
}

export function userOk(o: Outcome): bigint {
  return o.intoMatch({
    Ok: (v) => {
      const t = v._0;
      try {
        return t.n;
      } finally {
        t.drop();
      }
    },
    Other: () => {
      return 0n;
    },
  });
}

export function firstOk<I extends Iterable<Result<Token, Token>>>(items: SeqCursor<Result<Token, Token>>): bigint {
  try {
    {
      const _v1 = items.next();
      if (_v1 != null) {
        const r = _v1;
        {
          const _v = r;
          if (_v.isOk()) {
            const t = _v.unwrap();
            try {
              return t.n;
            } finally {
              t.drop();
            }
          } else {
          _v.drop();
          return 0n;
        }
        }
      } else {
      return 0n;
    }
    }
  } finally {
    items.drop();
  }
}

