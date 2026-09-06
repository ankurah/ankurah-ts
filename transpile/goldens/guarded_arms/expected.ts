// MIRRORS: ankurah/guarded_arms/src/input.rs
import { Struct, Enum, Result, Mutex, dropOwned, OwnershipFatal, UnsupportedShape, dropUnbound, unsupported, checkedAdd, checkedMul } from '@ankurah/base';

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
        let _g3;
        try {
          _g3 = token.n > 0;
        } catch (_e) {
          if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
          token.drop();
          dropUnbound(v, ['_0']);
          throw _e;
        }
        if (_g3) {
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

export function settle(r: Result<number, Refusal>, cached: boolean): Result<number, Refusal> {
  if (r.isOk()) {
    const n = r.unwrap();
    return Result.Ok(n);
  } else {
    const _v = r.unwrapErr();
    if (_v.is('Empty')) {
      const _v1 = _v;
      let _g0;
      try {
        _g0 = cached;
      } catch (_e) {
        if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
        _v1.drop();
        throw _e;
      }
      if (_g0) {
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
      let _g0;
      try {
        _g0 = cached;
      } catch (_e) {
        if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
        _v.drop();
        throw _e;
      }
      if (_g0) {
        dropOwned(_v);
        unsupported('an arm of this `Result` match tests INSIDE the payload and takes a DROPPABLE name out of it, and the port cannot both take a name out of a payload and release what is left of it here');
      }
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

export function guardTakesALock(input: Guarded, cell: Mutex<number>): number {
  return input.intoMatch({
    Same: (v) => {
      {
        const token = v._0;
        let _g4;
        try {
          const _t0 = cell.lock();
          try {
            _g4 = _t0.value > 0;
          } finally {
            _t0.drop();
          }
        } catch (_e) {
          if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
          token.drop();
          dropUnbound(v, ['_0']);
          throw _e;
        }
        if (_g4) {
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
            const _t3 = cell.lock();
            try {
              const seen = _t3.value;
              _t3.drop();
              _moved2 = true;
              token.drop();
              return checkedAdd(seen, 2, 'u32');
            } finally {
              _t3.drop();
            }
          } finally {
            if (!_moved2) token.drop();
          }
        } finally {
          dropUnbound(v, ['_0']);
        }
      }
    },
    Other: () => 0,
  });
}

export function refuses(n: number): boolean {
  if (n === 0) {
    throw new Error('the guard refuses zero');
  } else {
    return true;
  }
}

export function guardPanics(input: Guarded): number {
  return input.intoMatch({
    Same: (v) => {
      {
        const token = v._0;
        let _g2;
        try {
          _g2 = refuses(token.n);
        } catch (_e) {
          if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
          token.drop();
          dropUnbound(v, ['_0']);
          throw _e;
        }
        if (_g2) {
          try {
            let _moved0 = false;
            try {
              _moved0 = true;
              token.drop();
              return 1;
            } finally {
              if (!_moved0) token.drop();
            }
          } finally {
            dropUnbound(v, ['_0']);
          }
        }
      }
      {
        const token = v._0;
        try {
          let _moved1 = false;
          try {
            _moved1 = true;
            token.drop();
            return 2;
          } finally {
            if (!_moved1) token.drop();
          }
        } finally {
          dropUnbound(v, ['_0']);
        }
      }
    },
    Other: () => 0,
  });
}

export function guardedCatchAll(input: Guarded, flag: boolean): number {
  const _h0 = input;
  _h0.drop();
  unsupported('an arm of this `match` has a guard and the match hands its payload to the arms, and the if-chain a guard needs reads the subject without marking it moved; no form of this match is written');
}

export async function slow(n: number): Promise<boolean> {
  return n > 0;
}

export async function awaitedGuard(input: Guarded): Promise<number> {
  return await (input.intoMatch({
    Same: async (v) => {
      {
        const token = v._0;
        let _g2;
        try {
          _g2 = await slow(token.n);
        } catch (_e) {
          if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
          token.drop();
          dropUnbound(v, ['_0']);
          throw _e;
        }
        if (_g2) {
          try {
            let _moved0 = false;
            try {
              _moved0 = true;
              token.drop();
              return 1;
            } finally {
              if (!_moved0) token.drop();
            }
          } finally {
            dropUnbound(v, ['_0']);
          }
        }
      }
      {
        const token = v._0;
        try {
          let _moved1 = false;
          try {
            _moved1 = true;
            token.drop();
            return 2;
          } finally {
            if (!_moved1) token.drop();
          }
        } finally {
          dropUnbound(v, ['_0']);
        }
      }
    },
    Other: async () => 0,
  }));
}

