// MIRRORS: ankurah/a_local_typed_inside_an_adaptor/src/input.rs
import { Struct, dropOwned, checkedAdd } from '@ankurah/base';

export class Thing extends Struct {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }

  static new(id: bigint): Thing {
    return new Thing(id);
  }

  wrap(): Tag {
    return new Tag(this.id);
  }
}

export class Tag extends Struct {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }
}

export function totalOf(items: Thing[], stop: boolean): bigint {
  try {
    let _moved0 = false;
    let kept: Tag[] = [];
    try {
      const ids = [...items].map((t) => {
        kept.push(t.wrap());
        return t.id;
      });
      if (stop) {
        return BigInt(ids.length);
      }
      let total = 0n;
      _moved0 = true;
      const _seq1 = kept;
      let _at2 = 0;
      try {
        while (_at2 < _seq1.length) {
          const k = _seq1[_at2++];
          try {
            total = checkedAdd(total, k.id, 'u64');
          } finally {
            k.drop();
          }
        }
      } finally {
        dropOwned(_seq1.slice(_at2));
      }
      return total;
    } finally {
      if (!_moved0) dropOwned(kept);
    }
  } finally {
    dropOwned(items);
  }
}

export function run(): bigint {
  const taken = totalOf([Thing.new(1n), Thing.new(2n)], false);
  const left = totalOf([Thing.new(3n), Thing.new(4n)], true);
  return checkedAdd(taken, left, 'u64');
}

