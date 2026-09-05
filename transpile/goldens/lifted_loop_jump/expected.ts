// MIRRORS: ankurah/lifted_loop_jump/src/input.rs
import { Enum, Result, BorrowMut } from '@ankurah/base';

export type RefusalV = {
  Empty: {};
};

export class Refusal extends Enum<RefusalV> {
}

export type LitV = {
  Text: { _0: string };
  Count: { _0: number };
};

export class Lit extends Enum<LitV> {
}

export function quote(lit: Lit, out: BorrowMut<string>): Result<number, Refusal> {
  const _m0 = lit.match<any>({
    Text: (v) => {
      const s = v._0;
      out.value += '\'';
      for (const c of [...s]) {
        if (c === '\u{0}') {
          continue;
        }
        out.value += c;
      }
      out.value += '\'';
    },
    Count: (v) => {
      const n = v._0;
      if (n === 0) {
        return { $jump: 'return', $value: Result.Err(new Refusal('Empty', {})) };
      }
      out.value += 'n';
    },
  });
  if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
  return Result.Ok(out.value.length);
}

export function quoteAll(lits: Lit[], out: BorrowMut<string>): Result<number, Refusal> {
  rows: for (const lit of lits) {
    if (lit.is('Text')) {
      const { _0: s } = lit.value;
      for (const c of [...s]) {
        if (c === '!') {
          break rows;
        }
        out.value += c;
      }
    } else {
      const { _0: n } = lit.value;
      if (n === 0) {
        return Result.Err(new Refusal('Empty', {}));
      }
      out.value += 'n';
    }
  }
  return Result.Ok(out.value.length);
}

