// MIRRORS: ankurah/loop_frame/src/input.rs
import { Struct, dropOwned, unsupported, checkedAdd } from '@ankurah/base';

export class Token extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export function look(t: Token): number {
  return t.n;
}

export function shadowed(xs: Token[], replacement: Token[]): number {
  const xs_1 = xs;
  let total = 0;
  const _seq0 = xs_1;
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const item = _seq0[_at1++];
      try {
        total = checkedAdd(total, look(item), 'i32');
      } finally {
        item.drop();
      }
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  const xs_2 = replacement;
  try {
    const _built = unsupported('`collect` into `BinaryHeap<number>` is a `FromIterator` the port has no construction for');
    return total;
  } finally {
    dropOwned(xs_2);
  }
}

export function twice(a: Token[], b: Token[]): number {
  let total = 0;
  const _seq0 = a;
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const rest = _seq0[_at1++];
      try {
        total = checkedAdd(total, look(rest), 'i32');
      } finally {
        rest.drop();
      }
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  const _seq2 = b;
  let _at3 = 0;
  try {
    while (_at3 < _seq2.length) {
      const rest = _seq2[_at3++];
      try {
        total = checkedAdd(total, look(rest), 'i32');
      } finally {
        rest.drop();
      }
    }
  } finally {
    dropOwned(_seq2.slice(_at3));
  }
  return total;
}

