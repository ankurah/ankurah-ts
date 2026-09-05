// MIRRORS: ankurah/value_positions/src/input.rs
import { Enum, Result, checkedAdd, checkedSub, checkedRem } from '@ankurah/base';

export type RefusalV = {
  Empty: {};
};

export class Refusal extends Enum<RefusalV> {
}

function checked(n: number): Result<number, Refusal> {
  if (n === 0) {
    return Result.Err(new Refusal('Empty', {}));
  } else {
    return Result.Ok(n);
  }
}

export function firstEven(n: number): number {
  let _lv0;
  _at1: while (true) {
    if (checkedRem(n, 2, 'u32') === 0) {
      _lv0 = n;
      break _at1;
    }
    n = checkedAdd(n, 1, 'u32');
  }
  const found = _lv0;
  return checkedAdd(found, 1, 'u32');
}

export function pick(ok: boolean): number {
  const n = (ok ? 1 : 2);
  return checkedAdd(n, 1, 'i32');
}

export function untilZero(v: number[]): number {
  let total = 0;
  for (const x of v) {
    const _m0 = (() => {
      if (x === 0) {
        return { $jump: 'break' };
      } else {
        return x;
      }
    })();
    if ((_m0 as any)?.$jump === 'break') break;
    total = checkedAdd(total, (_m0 as any), 'u32');
  }
  return total;
}

export function total(v: number[]): Result<number, Refusal> {
  let sum = 0;
  for (const x of v) {
    if (x === 0) {
      sum = checkedAdd(sum, 1, 'u32');
    } else {
      const n = x;
      const _r0 = checked(n);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      sum = checkedAdd(sum, _r0.unwrap(), 'u32');
    }
  }
  return Result.Ok(sum);
}

export function operand(yes: boolean): number {
  return checkedAdd((yes ? 1 : 2), 3, 'u32');
}

export function compared(a: number, yes: boolean): boolean {
  return a === (yes ? 1 : 2);
}

function sink(n: number): number {
  return n;
}

export function jumpInAnArgument(stopAt: number): number {
  let total = 0;
  outer: while (true) {
    const _m0 = (() => {
      if (total >= stopAt) {
        return { $jump: 'break', $label: 'outer' };
      } else {
        return checkedAdd(total, 1, 'u32');
      }
    })();
    if ((_m0 as any)?.$jump === 'break' && (_m0 as any)?.$label === 'outer') break outer;
    total = sink((_m0 as any));
  }
  return total;
}

export function jumpInABlockArgument(stop: boolean): number {
  let total = 0;
  outer: while (true) {
    if (stop) {
      const _m0 = (() => {
        return { $jump: 'break', $label: 'outer' };
      })();
      if ((_m0 as any)?.$jump === 'break' && (_m0 as any)?.$label === 'outer') break outer;
      total = sink((_m0 as any));
    }
    total = checkedAdd(total, 1, 'u32');
  }
  return total;
}

export function firstEvenTail(n: number): number {
  let _lv0;
  _at1: while (true) {
    if (checkedRem(n, 2, 'u32') === 0) {
      _lv0 = n;
      break _at1;
    }
    n = checkedAdd(n, 1, 'u32');
  }
  return _lv0;
}

export function spin(n: number): number {
  let seen = 0;
  while (true) {
    if (n === 0) {
      break;
    }
    n = checkedSub(n, 1, 'u32');
    seen = checkedAdd(seen, 1, 'u32');
  }
  return seen;
}

