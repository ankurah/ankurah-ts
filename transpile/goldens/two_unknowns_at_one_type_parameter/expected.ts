// MIRRORS: ankurah/two_unknowns_at_one_type_parameter/src/input.rs
import { Struct, dropOwned } from '@ankurah/base';

export class Tag extends Struct {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  static new(n: bigint): Tag {
    return new Tag(n);
  }
}

export function moveOne<T>(from: T[], into: T[]): void {
  {
    const _v = from.pop();
    if (_v != null) {
      const item = _v;
      into.push(item);
    }
  }
}

export function moved(): number {
  let source = [];
  try {
    source.push(Tag.new(4n));
    let taken: Tag[] = [];
    try {
      moveOne(source, taken);
      return taken.length;
    } finally {
      dropOwned(taken);
    }
  } finally {
    dropOwned(source);
  }
}

