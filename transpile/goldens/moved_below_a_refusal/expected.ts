// MIRRORS: ankurah/moved_below_a_refusal/src/input.rs
import { Struct, Drop, dropOwned, unsupported, checkedAdd } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function take(t: Token): bigint {
  try {
    return t.n;
  } finally {
    t.drop();
  }
}

export function inALoop(b: Token[]): bigint {
  let total = 0n;
  const _seq1 = b;
  let _at2 = 0;
  try {
    while (_at2 < _seq1.length) {
      const rest = _seq1[_at2++];
      try {
        const h = unsupported('`collect` into `BinaryHeap<bigint>` is a `FromIterator` the port has no construction for');
        const _ = h;
        total = checkedAdd(total, take(rest), 'i64');
      } finally {
        rest.drop();
      }
    }
  } finally {
    dropOwned(_seq1.slice(_at2));
  }
  return total;
}

export function aParameter(t: Token): bigint {
  try {
    const h = unsupported('`collect` into `BinaryHeap<bigint>` is a `FromIterator` the port has no construction for');
    const _ = h;
    return take(t);
  } finally {
    t.drop();
  }
}

