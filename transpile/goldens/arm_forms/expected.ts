// MIRRORS: ankurah/arm_forms/src/input.rs
import { Struct, Enum, OwnershipFatal, UnsupportedShape, dropUnbound, checkedAdd, checkedMul } from '@ankurah/base';

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

export type SourceV = {
  Given: { _0: number };
  Absent: {};
};

export class Source extends Enum<SourceV> {
}

export type AnswerV = {
  Number: { _0: number };
  Missing: {};
};

export class Answer extends Enum<AnswerV> {
}

export type WeightV = {
  Light: { _0: number };
  Heavy: { _0: number };
};

export class Weight extends Enum<WeightV> {
}

export type HolderV = {
  One: { _0: Token };
  Two: { _0: Token };
};

export class Holder extends Enum<HolderV> {
}

export function resolve(source: Source, fallback: number | null): Answer {
  try {
    return source.match({
      Given: (v) => {
        const n = v._0;
        return new Answer('Number', { _0: n });
      },
      Absent: () => {
        if (fallback != null) {
          const n = fallback;
          return new Answer('Number', { _0: n });
        } else {
          return new Answer('Missing', {});
        }
      },
    });
  } finally {
    source.drop();
  }
}

export function resolveTwice(source: Source, fallback: number | null, floor: number | null): Answer {
  try {
    return source.match({
      Given: (v) => {
        const n = v._0;
        return new Answer('Number', { _0: n });
      },
      Absent: () => {
        if (fallback != null) {
          const n = fallback;
          return new Answer('Number', { _0: n });
        } else {
          if (floor != null) {
            const n = floor;
            return new Answer('Number', { _0: n });
          } else {
            return new Answer('Missing', {});
          }
        }
      },
    });
  } finally {
    source.drop();
  }
}

export function record(w: Weight, into: number[]): number {
  _match0: {
    if (w.is('Light')) {
      const { _0: n } = w.value;
      if (n > 3) {
        into.push(n);
        if (n > 100) {
          return 1;
        }
        break _match0;
      }
    }
    if (w.is('Light')) {
      into.push(0);
      break _match0;
    }
    {
      const { _0: n } = w.value;
      into.push(n);
    }
  }
  return 0;
}

export function weigh(input: Weight, floor: number): number {
  try {
    if (input.is('Light')) {
      const { _0: n } = input.value;
      if (n > floor) {
        return n;
      }
    }
    if (input.is('Light')) {
      const { _0: n } = input.value;
      return floor;
    }
    {
      const { _0: n } = input.value;
      return checkedMul(n, 2, 'u32');
    }
  } finally {
    input.drop();
  }
}

export function tally(input: Source, token: Token, floor: number | null): number {
  try {
    const answer = (() => {
      if (input.is('Given')) {
        const { _0: n } = input.value;
        if (n > 0) {
          return n;
        }
      }
      if (input.is('Given')) {
        if (floor != null) {
          const n = floor;
          return n;
        } else {
          return 0;
        }
      }
      {
        return 0;
      }
    })();
    const total = checkedAdd(answer, token.n, 'i32');
    token.drop();
    return total;
  } finally {
    input.drop();
  }
}

export function pick(input: Holder, floor: number): number {
  return input.intoMatch({
    One: (v) => {
      {
        const t = v._0;
        let _g0;
        try {
          _g0 = t.n > floor;
        } catch (_e) {
          if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
          t.drop();
          throw _e;
        }
        if (_g0) {
          try {
            if (t.n > 100) {
              return 100;
            } else {
              return t.n;
            }
          } finally {
            t.drop();
          }
        }
      }
      {
        try {
          return floor;
        } finally {
          dropUnbound(v, []);
        }
      }
    },
    Two: (v) => {
      const t = v._0;
      try {
        if (floor === 0) {
          return t.n;
        } else {
          return floor;
        }
      } finally {
        t.drop();
      }
    },
  });
}

