// MIRRORS: ankurah/question_from/src/input.rs
import { Struct, Result } from '@ankurah/base';

export class Wire extends Struct {
  readonly code: number;

  constructor(code: number) {
    super();
    this.code = code;
  }
}

export class Wrapped extends Struct {
  readonly code: number;
  readonly context: string;

  constructor(code: number, context: string) {
    super();
    this.code = code;
    this.context = context;
  }

  static fromWire(wire: Wire): Wrapped {
    try {
      return new Wrapped(wire.code, 'wire');
    } finally {
      wire.drop();
    }
  }
}

export function read(raw: string): Result<number, Wire> {
  if (raw.length === 0) {
    return Result.Err(new Wire(7));
  }
  return Result.Ok(raw.length);
}

export function wrapped(raw: string): Result<number, Wrapped> {
  const _r0 = read(raw);
  if (_r0.isErr()) return Result.Err(Wrapped.fromWire(_r0.unwrapErr()));
  const n = _r0.unwrap();
  return Result.Ok(n + 1);
}

export function passedOn(raw: string): Result<number, Wire> {
  const _r0 = read(raw);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const n = _r0.unwrap();
  return Result.Ok(n + 1);
}

export function doubled(raw: string): Result<number, Wrapped> {
  const _r0 = wrapped(raw);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  return Result.Ok(_r0.unwrap() * 2);
}

