// MIRRORS: ankurah/a_suspension_before_the_read/src/input.rs
import { Struct, Drop, Result, dropOwned, checkedAdd } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function pass(t: Token): Result<Token, string> {
  return Result.Ok(t);
}

export function eat(t: Token): bigint {
  try {
    return t.n;
  } finally {
    t.drop();
  }
}

export function pair(a: bigint, b: bigint): bigint {
  return checkedAdd(a, b, 'i64');
}

export function passN(n: bigint): Result<bigint, string> {
  return Result.Ok(n);
}

export async function step(n: bigint): Promise<bigint> {
  return n;
}

export async function suspendsFirst(t: Token, f: Promise<bigint>): Promise<Result<bigint, string>> {
  const _r0 = pass(t);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  try {
    return Result.Ok(pair(await f, eat(_r0.unwrap())));
  } finally {
    if (_r0 != null && !(_r0 as any).isMoved && !(_r0 as any).isDropped) dropOwned(_r0);
  }
}

export async function suspendsLast(n: bigint): Promise<Result<bigint, string>> {
  const _r0 = passN(n);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  return Result.Ok(pair(0n, await step(_r0.unwrap())));
}

