// MIRRORS: ankurah/contested_variant/src/input.rs
import { Struct, Enum, Result, dropUnbound, checkedAdd } from '@ankurah/base';

export class Payload extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export type LitV = {
  Flag: { _0: boolean };
  Count: { _0: number };
};

export class Lit extends Enum<LitV> {
}

export type ExprV = {
  Literal: { _0: Lit };
  Held: { _0: Payload };
  Nothing: {};
};

export class Expr extends Enum<ExprV> {

  widthOf(): number {
    return this.match({
      Literal: (v) => {
        if (v._0.is('Count')) {
          const { _0: n } = v._0.value;
          return n;
        } else {
          return 1;
        }
      },
      Held: (v) => {
        const p = v._0;
        return p.n;
      },
      Nothing: () => 0,
    });
  }
}

export type StepV = {
  Ready: { _0: Payload | null };
  Pending: {};
};

export class Step extends Enum<StepV> {
}

export interface Widths {
  widthOf(): number;
}

export function truthy(e: Expr): Result<boolean, string> {
  try {
    if (e.is('Literal') && (e.value._0.is('Flag') && (e.value._0.value._0 === true))) {
      return Result.Ok(true);
    } else if (e.is('Literal') && (e.value._0.is('Flag') && (e.value._0.value._0 === false))) {
      return Result.Ok(false);
    } else {
      return Result.Err('not a flag');
    }
  } finally {
    e.drop();
  }
}

export function takeOne(step: Step, into: number[]): boolean {
  return step.intoMatch({
    Ready: (v) => {
      if (v._0 != null) {
        const item = v._0;
        try {
          into.push(item.n);
          return true;
        } finally {
          item.drop();
        }
      } else {
        try {
          return false;
        } finally {
          dropUnbound(v, []);
        }
      }
    },
    Pending: () => false,
  });
}

export function describe(e: Expr): string {
  return e.match({
    Literal: (v) => {
      if (v._0.is('Flag')) {
        return 'flag';
      } else {
        return 'count';
      }
    },
    Held: (v) => 'held',
    Nothing: () => 'nothing',
  });
}

function width(e: Expr): Result<number, string> {
  if (e.is('Literal') && (e.value._0.is('Count'))) {
    const { _0: n } = e.value._0.value;
    return Result.Ok(n);
  } else {
    return Result.Err('no width');
  }
}

export function widen(e: Expr, source: Expr): Result<number, string> {
  return e.intoMatch({
    Literal: (v) => {
      if (v._0.is('Flag') && (v._0.value._0 === true)) {
        try {
          const _r0 = width(source);
          if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
          const n = _r0.unwrap();
          return Result.Ok(checkedAdd(n, 1, 'u32'));
        } finally {
          dropUnbound(v, []);
        }
      } else if (v._0.is('Flag') && (v._0.value._0 === false)) {
        try {
          return Result.Err('false');
        } finally {
          dropUnbound(v, []);
        }
      } else if (v._0.is('Count')) {
        const { _0: n } = v._0.value;
        try {
          return Result.Ok(n);
        } finally {
          dropUnbound(v, []);
        }
      } else {
        try {
          return Result.Err('no');
        } finally {
          dropUnbound(v, []);
        }
      }
    },
    Held: (v) => {
      try {
        return Result.Err('no');
      } finally {
        dropUnbound(v, []);
      }
    },
    Nothing: () => Result.Err('no'),
  });
}

function nextStep(items: Payload[]): Step {
  const _v = items.pop();
  if (_v != null) {
    const p = _v;
    return new Step('Ready', { _0: p });
  } else {
    return new Step('Ready', { _0: null });
  }
}

export function drain(items: Payload[], into: number[]): number {
  let turns = 0;
  while (true) {
    const _m0 = nextStep(items).intoMatch<any>({
      Ready: (v) => {
        if (v._0 != null) {
          const item = v._0;
          try {
            into.push(item.n);
            turns = checkedAdd(turns, 1, 'u32');
          } finally {
            item.drop();
          }
        } else {
          try {
            return { $jump: 'break' }
          } finally {
            dropUnbound(v, []);
          }
        }
      },
      Pending: () => {
        return { $jump: 'break' }
      },
    });
    if ((_m0 as any)?.$jump === 'break') break;
  }
  return turns;
}

export function widest<W extends Widths>(a: W, b: W): number {
  if (a.widthOf() >= b.widthOf()) {
    return a.widthOf();
  } else {
    return b.widthOf();
  }
}

