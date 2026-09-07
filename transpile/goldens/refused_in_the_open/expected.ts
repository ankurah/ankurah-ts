// MIRRORS: ankurah/refused_in_the_open/src/input.rs
import { Struct, Drop, invokeRef, Invocable, dropOwned, unsupported, checkedAdd } from '@ankurah/base';

export class Token extends Drop {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function take2(a: Token, b: number): number {
  try {
    const n = a.n;
    return checkedAdd(n, b, 'u32');
  } finally {
    a.drop();
  }
}

export function apply(f: Invocable<[number], number>): number {
  try {
    return invokeRef(f, 1);
  } finally {
    dropOwned(f);
  }
}

export function blockExpression(held: Token, rest: number[]): number {
  try {
    return checkedAdd(((() => {
      const _h = unsupported('`collect` into `BinaryHeap<number>` is a `FromIterator` the port has no construction for');
      return 0;
    })()), take2(held, 1), 'u32');
  } finally {
    held.drop();
  }
}

export function arrowInAString(held: Token, rest: number[]): number {
  try {
    return checkedAdd(checkedAdd('=>'.length, ((() => {
      const _h = unsupported('`collect` into `BinaryHeap<number>` is a `FromIterator` the port has no construction for');
      return 0;
    })()), 'u32'), take2(held, 1), 'u32');
  } finally {
    held.drop();
  }
}

export function besideAClosure(held: Token, rest: number[]): number {
  try {
    return checkedAdd(checkedAdd(apply((x) => checkedAdd(x, 1, 'u32')), ((() => {
      const _h = unsupported('`collect` into `BinaryHeap<number>` is a `FromIterator` the port has no construction for');
      return 0;
    })()), 'u32'), take2(held, 1), 'u32');
  } finally {
    held.drop();
  }
}

export function insideACallable(held: Token): number {
  return checkedAdd(take2(held, 1), apply((x) => {
    const _h = unsupported('`collect` into `BinaryHeap<number>` is a `FromIterator` the port has no construction for');
    return 0;
  }), 'u32');
}

