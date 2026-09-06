// MIRRORS: ankurah/negation_and_rest/src/input.rs
import { Enum, checkedNeg } from '@ankurah/base';

export type WideV = {
  Two: { _0: number; _1: number };
  One: { _0: number };
  Nothing: {};
};

export class Wide extends Enum<WideV> {
}

export function negate(n: number): number {
  return checkedNeg(n, 'i32');
}

export function negateWide(n: bigint): bigint {
  return checkedNeg(n, 'i64');
}

export function negateFloat(x: number): number {
  return -x;
}

export function smallest(): number {
  return -2147483648;
}

export function covered(w: Wide): number {
  return w.match({
    Two: (v) => 2,
    One: (v) => {
      const n = v._0;
      return n;
    },
    Nothing: () => 0,
  });
}

export function firstOf(w: Wide): number {
  return w.match({
    Two: (v) => {
      const a = v._0;
      return a;
    },
    One: (v) => {
      const n = v._0;
      return n;
    },
    Nothing: () => 0,
  });
}

