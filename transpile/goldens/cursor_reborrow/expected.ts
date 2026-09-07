// MIRRORS: ankurah/cursor_reborrow/src/input.rs
import { Struct, Drop, dropOwned, checkedAdd, SeqCursor } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function sumOf<I extends Iterable<Token>>(walk: SeqCursor<Token>): bigint {
  try {
    let total = 0n;
    const _seq0 = walk.drainRest();
    let _at1 = 0;
    try {
      while (_at1 < _seq0.length) {
        const token = _seq0[_at1++];
        try {
          total = checkedAdd(total, token.n, 'i64');
        } finally {
          token.drop();
        }
      }
    } finally {
      dropOwned(_seq0.slice(_at1));
    }
    return total;
  } finally {
    walk.drop();
  }
}

export function drain<I extends Iterable<Token>>(values: SeqCursor<Token>): Token[] {
  return values.drainRest();
}

export function drained<I extends Iterable<Token>>(walk: SeqCursor<Token>): bigint {
  try {
    const rest = drain(walk);
    let total = 0n;
    const _seq0 = rest;
    let _at1 = 0;
    try {
      while (_at1 < _seq0.length) {
        const token = _seq0[_at1++];
        try {
          total = checkedAdd(total, token.n, 'i64');
        } finally {
          token.drop();
        }
      }
    } finally {
      dropOwned(_seq0.slice(_at1));
    }
    return total;
  } finally {
    walk.drop();
  }
}

export function summed(tokens: Token[]): bigint {
  return sumOf(new SeqCursor([...tokens]));
}

export function allDrained(tokens: Token[]): bigint {
  return drained(new SeqCursor([...tokens]));
}

