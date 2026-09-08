// MIRRORS: ankurah/what_the_text_evaluates/src/input.rs
import { Struct, Drop, RefCell, rangeContains } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }

  equals(other: Token): boolean {
    return this.n === other.n;
  }

  partialCompareTo(other: Token): number | null {
    return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(this.n, other.n);
  }
}

export function look(t: Token): bigint {
  return t.n;
}

export function note(log: RefCell<bigint[]>, n: bigint): Token {
  const _t0 = log.borrowMut();
  try {
    _t0.value.push(n);
  } finally {
    _t0.drop();
  }
  return new Token(n);
}

export function number(log: RefCell<bigint[]>, n: bigint): bigint {
  const _t0 = log.borrowMut();
  try {
    _t0.value.push(n);
  } finally {
    _t0.drop();
  }
  return n;
}

export function inRange(log: RefCell<bigint[]>): boolean {
  const _t0 = note(log, 1n);
  try {
    const _t1 = note(log, 9n);
    try {
      const _t2 = note(log, 5n);
      try {
        return rangeContains(_t0, _t1, false, _t2);
      } finally {
        _t2.drop();
      }
    } finally {
      _t1.drop();
    }
  } finally {
    _t0.drop();
  }
}

export function repeated(log: RefCell<bigint[]>): bigint[] {
  const _m0 = number(log, 7n);
  return Array(Number(BigInt.asUintN(32, number(log, 2n)))).fill(_m0);
}

export function doubled(log: RefCell<bigint[]>): bigint {
  const _t0 = note(log, 3n);
  try {
    return look(_t0);
  } finally {
    _t0.drop();
  }
}

