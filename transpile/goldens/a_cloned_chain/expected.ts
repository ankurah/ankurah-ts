// MIRRORS: ankurah/a_cloned_chain/src/input.rs
import { Struct, Drop, dropOwned, derivedClone, checkedAdd } from '@ankurah/base';

export class Item extends Drop {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }

  protected override onDrop(): void {

  }

  clone(): Item {
    return new Item(this.id);
  }
}

export function keptOver(items: Item[]): bigint {
  const taken = [...[...items].filter((i) => i.id > 1n)].map((e) => derivedClone(e));
  let total = 0n;
  const _seq0 = taken;
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const t = _seq0[_at1++];
      try {
        total = checkedAdd(total, t.id, 'u64');
      } finally {
        t.drop();
      }
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  return total;
}

export function keptWhole(items: Item[]): bigint {
  const taken = [...[...items]].map((e) => e.clone());
  let total = 0n;
  const _seq0 = taken;
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const t = _seq0[_at1++];
      try {
        total = checkedAdd(total, t.id, 'u64');
      } finally {
        t.drop();
      }
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  return total;
}

