// MIRRORS: ankurah/value_positions/src/input.rs
import { Enum, Result, checkedAdd, checkedRem } from '@ankurah/base';

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
  const n = ok ? 1 : 2;
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

