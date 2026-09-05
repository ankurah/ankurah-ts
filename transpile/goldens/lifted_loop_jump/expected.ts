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

export function firstOver(rows: number[][], limit: number): number {
  let _lv0;
  outer: while (true) {
    for (const row of rows) {
      for (const cell of row) {
        const _m1 = (() => {
          {
            if (cell > limit) {
              return { $jump: 'break', $label: 'outer', $value: cell };
            }
            return 0;
          }
        })();
        if ((_m1 as any)?.$jump === 'break' && (_m1 as any)?.$label === 'outer') { _lv0 = (_m1 as any).$value; break outer; };
        const scaled = -(_m1 as any);
        if (scaled < -100) {
          _lv0 = 0;
          break outer;
        }
      }
    }
    _lv0 = 0;
    break outer;
  }
  return _lv0;
}

