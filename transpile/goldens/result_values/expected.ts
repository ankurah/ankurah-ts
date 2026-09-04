// MIRRORS: ankurah/result_values/src/input.rs
import { Enum, Result } from '@ankurah/base';

export type WireErrorV = {
  Truncated: {};
};

export class WireError extends Enum<WireErrorV> {

  clone(): WireError {
    return new WireError(this.type, { ...this.value });
  }

  equals(other: WireError): boolean {
    return true;
  }

  debug(): string {
    return this.match({
      Truncated: () => 'Truncated',
    });
  }
}

export function width(raw: string): Result<number, WireError> {
  if (raw.length === 0) {
    return Result.Err(new WireError('Truncated', {}));
  }
  return Result.Ok(raw.length);
}

export function bound(raw: string): Result<number, WireError> {
  const _r0 = width(raw);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const n = _r0.unwrap();
  return Result.Ok(n + 1);
}

export function insideAnExpression(raw: string): Result<number, WireError> {
  const _r0 = width(raw);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  return Result.Ok(_r0.unwrap() + 1);
}

export function discarded(raw: string): Result<number, WireError> {
  const _r0 = width(raw);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  _r0.drop();
  return Result.Ok(0);
}

export function defaulted(raw: string): number {
  return width(raw).unwrapOr(0);
}

