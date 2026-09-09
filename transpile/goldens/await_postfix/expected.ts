// MIRRORS: ankurah/await_postfix/src/input.rs
import { Struct, invokeRef, Invocable, dropOwned, checkedMul } from '@ankurah/base';

export class Holder extends Struct {
  readonly items: number[];

  constructor(items: number[]) {
    super();
    this.items = items;
  }
}

export async function getVec(): Promise<number[]> {
  return [1, 2, 3];
}

export async function getFunction(): Promise<Invocable<[number], number>> {
  return double;
}

export function double(n: number): number {
  return checkedMul(n, 2, 'u32');
}

export async function getHolder(): Promise<Holder> {
  return new Holder([4, 5]);
}

export async function first(): Promise<number> {
  return (await getVec())[0];
}

export async function tail(): Promise<number[]> {
  return (await getVec()).slice(1).slice();
}

export async function through(): Promise<number> {
  const _t0 = await getFunction();
  try {
    return invokeRef(_t0, 8);
  } finally {
    dropOwned(_t0);
  }
}

export async function width(): Promise<number> {
  return (await getVec()).length;
}

export async function held(): Promise<number[]> {
  const _t0 = (await getHolder());
  try {
    return _t0.items;
  } finally {
    _t0.drop();
  }
}

