// MIRRORS: ankurah/a_borrow_kept_in_a_collection/src/input.rs
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
}

export function totalOver(items: Thing[]): bigint {
  let picked = [];
  for (const t of items) {
    if (t.id > 1n) {
      picked.push(t);
    }
  }
  let total = 0n;
  for (const p of picked) {
    total = checkedAdd(total, p.id, 'u64');
  }
  return total;
}

export function kept(t: Thing): number {
  let picked = [];
  picked.push(t);
  return picked.length;
}

export function run(): bigint {
  const items = [Thing.new(1n), Thing.new(2n), Thing.new(3n)];
  try {
    const total = checkedAdd(totalOver(items), BigInt(kept(items[0])), 'u64');
    return total;
  } finally {
    dropOwned(items);
  }
}

