// MIRRORS: ankurah/guarded_arms/src/input.rs
import { Struct, Enum, Result, dropOwned, dropUnbound, unsupported, checkedMul } from '@ankurah/base';

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

export class Detail extends Struct {
  readonly why: string;

  constructor(why: string) {
    super();
    this.why = why;
  }
}

export type GuardedV = {
  Same: { _0: Token; _1: boolean };
  Other: {};
};

export class Guarded extends Enum<GuardedV> {
}

export type WeightV = {
  Light: { _0: number };
  Heavy: { _0: number };
};

export class Weight extends Enum<WeightV> {
}

export type RefusalV = {
  Empty: {};
  Late: {};
};

export class Refusal extends Enum<RefusalV> {
}

export type RichV = {
  Empty: { _0: Detail };
  Late: { _0: Detail };
};

export class Rich extends Enum<RichV> {
}

export function guardedConsuming(input: Guarded): number {
  return input.intoMatch({
    Same: (v) => {
      if (v._1 === true) {
        const token = v._0;
        if (token.n > 0) {
          try {
            let _moved1 = false;
            try {
              _moved1 = true;
              token.drop();
              return 1;
            } finally {
              if (!_moved1) token.drop();
            }
          } finally {
            dropUnbound(v, ['_0']);
          }
        }
      }
      {
        const token = v._0;
        try {
          let _moved2 = false;
          try {
            _moved2 = true;
            token.drop();
            return 2;
          } finally {
            if (!_moved2) token.drop();
          }
        } finally {
          dropUnbound(v, ['_0']);
        }
      }
    },
    Other: (v) => {
      const rest = new Guarded('Other', v);
      let _moved0 = false;
      try {
        _moved0 = true;
        rest.drop();
        return 0;
      } finally {
        if (!_moved0) rest.drop();
      }
    },
  });
}

export function heaviest(w: Weight): number {
  _match0: {
    if (w.is('Light')) {
      const { _0: n } = w.value;
      if (n > 10) {
        return 10;
      }
    }
    if (w.is('Light')) {
      const { _0: n } = w.value;
      return n;
    }
    {
      return 99;
    }
  }
}

export function settle(r: Result<number, Refusal>, cached: boolean): Result<number, Refusal> {
  if (r.isOk()) {
    const n = r.unwrap();
    return Result.Ok(n);
  } else {
    const _v = r.unwrapErr();
    if (_v.is('Empty')) {
      const _v1 = _v;
      if (cached) {
        try {
          return Result.Ok(0);
        } finally {
          _v1.drop();
        }
      }
    }
    {
      const e = _v;
      return Result.Err(e);
    }
  }
}

export function collect(n: number): Result<number[], Refusal> {
  if (n === 0) {
    return Result.Ok([]);
  } else {
    return Result.Ok([n]);
  }
}

export function bridge(n: number): number {
  const _v = collect(n);
  if (_v.isOk()) {
    const _v1 = _v.unwrap();
    {
      const events = _v1;
      if (!(events.length === 0)) {
        return events.length;
      }
    }
    {
      const _v2 = _v1;
      return 0;
    }
  } else {
    const _v3 = _v.unwrapErr();
    try {
      return 0;
    } finally {
      _v3.drop();
    }
  }
}

export function describe(w: Weight): string {
  try {
    _match0: {
      if (w.is('Light')) {
        const { _0: n } = w.value;
        if (n === 0) {
          return 'nothing';
        }
      }
      if (w.is('Light')) {
        const { _0: n } = w.value;
        return `light ${n}`;
      }
      {
        const { _0: n } = w.value;
        return `heavy ${n}`;
      }
    }
  } finally {
    w.drop();
  }
}

export function settleRich(r: Result<number, Rich>, cached: boolean): Result<number, Rich> {
  if (r.isOk()) {
    const n = r.unwrap();
    return Result.Ok(n);
  } else {
    const _v = r.unwrapErr();
    if (_v.is('Empty')) {
      dropOwned(_v);
      unsupported('an arm of this `Result` match tests INSIDE the payload and takes a DROPPABLE name out of it, and the port cannot both take a name out of a payload and release what is left of it here');
    }
    {
      const e = _v;
      return Result.Err(e);
    }
  }
}

export function count(w: Weight, into: number[]): void {
  _match0: {
    if (w.is('Light')) {
      const { _0: n } = w.value;
      if (n > 3) {
        {
          into.push(n);
        }
        break _match0;
      }
    }
    if (w.is('Light')) {
      {
        into.push(0);
      }
      break _match0;
    }
    {
      const { _0: n } = w.value;
      {
        into.push(checkedMul(n, 2, 'u32'));
      }
    }
  }
}

