// MIRRORS: ankurah/match_catch_all/src/input.rs
import { Struct, Enum, dropUnbound, checkedAdd, checkedMul } from '@ankurah/base';

export class Inner extends Struct {
  readonly width: number;

  constructor(width: number) {
    super();
    this.width = width;
  }
}

export type OrderV = {
  Less: {};
  Equal: {};
  Greater: {};
};

export class Order extends Enum<OrderV> {
}

export type CauseV = {
  Denied: { _0: Inner };
  Missing: {};
  Other: {};
};

export class Cause extends Enum<CauseV> {
}

export type WrappedV = {
  Held: { _0: Inner };
  Whole: { _0: Cause };
};

export class Wrapped extends Enum<WrappedV> {
}

export type HeldV = {
  First: { _0: Inner };
  Second: { _0: Inner };
  Third: { _0: Inner };
};

export class Held extends Enum<HeldV> {
}

export type ReasonV = {
  Cause: { _0: Cause };
  Plain: {};
};

export class Reason extends Enum<ReasonV> {
}

export function tieBreak(order: Order, fallback: Order): Order {
  let _moved0 = false;
  let _moved1 = false;
  try {
    try {
      return order.match({
        Equal: () => {
          _moved1 = true;
          return fallback;
        },
        Less: () => {
          _moved0 = true;
          const other = order;
          return other;
        },
        Greater: () => {
          _moved0 = true;
          const other = order;
          return other;
        },
      });
    } finally {
      if (!_moved1) fallback.drop();
    }
  } finally {
    if (!_moved0) order.drop();
  }
}

export function lift(cause: Cause): Wrapped {
  return cause.intoMatch({
    Denied: (v) => {
      const inner = v._0;
      return new Wrapped('Held', { _0: inner });
    },
    Missing: (v) => {
      const cause = new Cause('Missing', v);
      return new Wrapped('Whole', { _0: cause });
    },
    Other: (v) => {
      const cause = new Cause('Other', v);
      return new Wrapped('Whole', { _0: cause });
    },
  });
}

export function rank(cause: Cause): number {
  return cause.match({
    Denied: (v) => {
      const inner = v._0;
      return inner.width;
    },
    Missing: () => 1,
    Other: () => 0,
  });
}

export function widen(cause: Cause, into: number[]): void {
  return cause.match({
    Denied: (v) => {
      const inner = v._0;
      into.push(inner.width);
    },
    Missing: () => {
      into.push(0);
    },
    Other: () => {
      into.push(0);
    },
  });
}

export function count(cause: Cause): number {
  return cause.match({
    Denied: (v) => {
      const inner = v._0;
      return inner.width;
    },
    Missing: () => 1,
    Other: () => 1,
  });
}

export function tally(cause: Cause): number {
  return cause.intoMatch({
    Denied: (v) => {
      const inner = v._0;
      try {
        return inner.width;
      } finally {
        inner.drop();
      }
    },
    Missing: (v) => {
      const rest = new Cause('Missing', v);
      try {
        return count(rest);
      } finally {
        rest.drop();
      }
    },
    Other: (v) => {
      const rest = new Cause('Other', v);
      try {
        return count(rest);
      } finally {
        rest.drop();
      }
    },
  });
}

export function letInit(cause: Cause): number {
  const picked = cause.match({
    Denied: (v) => {
      const inner = v._0;
      return inner.width;
    },
    Missing: () => 2,
    Other: () => 2,
  });
  return checkedAdd(picked, 1, 'i32');
}

export function asArgument(cause: Cause): number {
  return countTwice(cause.match({
    Denied: (v) => {
      const inner = v._0;
      return inner.width;
    },
    Missing: () => 3,
    Other: () => 3,
  }));
}

function countTwice(n: number): number {
  return checkedMul(n, 2, 'usize');
}

export function ignore(held: Held): number {
  return held.intoMatch({
    First: (v) => {
      const inner = v._0;
      try {
        return inner.width;
      } finally {
        inner.drop();
      }
    },
    Second: (v) => {
      try {
        return 0;
      } finally {
        dropUnbound(v, []);
      }
    },
    Third: (v) => {
      try {
        return 0;
      } finally {
        dropUnbound(v, []);
      }
    },
  });
}

export function ignoreNamed(held: Held): number {
  return held.intoMatch({
    First: (v) => {
      try {
        return 1;
      } finally {
        dropUnbound(v, []);
      }
    },
    Second: (v) => {
      const inner = v._0;
      try {
        return inner.width;
      } finally {
        inner.drop();
      }
    },
    Third: (v) => {
      try {
        return 3;
      } finally {
        dropUnbound(v, []);
      }
    },
  });
}

export function refutable(reason: Reason): number {
  if (reason.is('Cause') && (reason.value._0.is('Missing'))) {
    return 5;
  } else {
    return 6;
  }
}

export function sameName(cause: Cause): Cause {
  return cause.intoMatch({
    Denied: (v) => {
      const inner = v._0;
      return new Cause('Denied', { _0: inner });
    },
    Missing: (v) => {
      const cause = new Cause('Missing', v);
      return cause;
    },
    Other: (v) => {
      const cause = new Cause('Other', v);
      return cause;
    },
  });
}

export function unwind(cause: Cause): number {
  return cause.intoMatch({
    Denied: (v) => {
      const inner = v._0;
      try {
        throw new Error(`width ${inner.width} is not allowed`);
      } finally {
        inner.drop();
      }
    },
    Missing: () => 0,
    Other: () => 0,
  });
}

