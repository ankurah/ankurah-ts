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
  let _moved0 = false;
  try {
    const xs_1 = xs;
    let total = 0;
    const _seq1 = xs_1;
    let _at2 = 0;
    try {
      while (_at2 < _seq1.length) {
        const item = _seq1[_at2++];
        try {
          total = checkedAdd(total, look(item), 'u32');
        } finally {
          item.drop();
        }
      }
    } finally {
      dropOwned(_seq1.slice(_at2));
    }
    _moved0 = true;
    const xs_2 = replacement;
    try {
      const _built = unsupported('`collect` into `BinaryHeap<number>` is a `FromIterator` the port has no construction for');
      return total;
    } finally {
      dropOwned(xs_2);
    }
  } finally {
    if (!_moved0) dropOwned(replacement);
  }
}

export function twice(a: Token[], b: Token[]): number {
  let _moved0 = false;
  try {
    let total = 0;
    const _seq1 = a;
    let _at2 = 0;
    try {
      while (_at2 < _seq1.length) {
        const rest = _seq1[_at2++];
        try {
          total = checkedAdd(total, look(rest), 'u32');
        } finally {
          rest.drop();
        }
      }
    } finally {
      dropOwned(_seq1.slice(_at2));
    }
    _moved0 = true;
    const _seq3 = b;
    let _at4 = 0;
    try {
      while (_at4 < _seq3.length) {
        const rest = _seq3[_at4++];
        try {
          total = checkedAdd(total, look(rest), 'u32');
        } finally {
          rest.drop();
        }
      }
    } finally {
      dropOwned(_seq3.slice(_at4));
    }
    return total;
  } finally {
    if (!_moved0) dropOwned(b);
  }
}

