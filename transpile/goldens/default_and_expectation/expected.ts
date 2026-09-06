// MIRRORS: ankurah/default_and_expectation/src/input.rs
import { Enum, Result, serde_json } from '@ankurah/base';

export type LitV = {
  Bool: { _0: boolean };
  Text: { _0: string };
};

export class Lit extends Enum<LitV> {
}

export function toBytes(l: Lit): Uint8Array {
  return l.match({
    Bool: (v) => {
      const b = v._0;
      return new Uint8Array([Number(b)]);
    },
    Text: (v) => new Uint8Array([0, 1]),
  });
}

export function textOrEmpty(s: string | null): string {
  return (s ?? '');
}

export function countOrZero(n: number | null): number {
  return (n ?? 0);
}

export function bytesOrEmpty(r: Result<Uint8Array, string>): Uint8Array {
  return r.unwrapOr(new Uint8Array());
}

export function jsonOf(flag: boolean): unknown {
  return flag;
}

export function jsonNull(): unknown {
  return null;
}

export function jsonBytes(v: unknown): Uint8Array {
  return serde_json.toVec(v).unwrapOr(new Uint8Array());
}

