// MIRRORS: ankurah/reserved_names/src/input.rs
import { Struct, unsupported, checkedAdd } from '@ankurah/base';

export class Holder extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  with(extra: number): number {
    return checkedAdd(this.n, extra, 'u32');
  }
}

export function unsupported_(_what: string): number {
  return 7;
}

export function with_(n: number): number {
  return checkedAdd(n, 1, 'u32');
}

export function in_(n: number): number {
  return checkedAdd(n, 2, 'u32');
}

export function callsThem(n: number): number {
  return checkedAdd(checkedAdd(with_(n), in_(n), 'u32'), unsupported_('nothing'), 'u32');
}

export function refuses(xs: number[]): number {
  const _h = unsupported('`collect` into `BinaryHeap<number>` is a `FromIterator` the port has no construction for');
  return 0;
}

