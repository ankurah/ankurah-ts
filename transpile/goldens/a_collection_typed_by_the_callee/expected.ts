// MIRRORS: ankurah/a_collection_typed_by_the_callee/src/input.rs
import { Struct, dropOwned, checkedAdd } from '@ankurah/base';

export class Tag extends Struct {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }

  static new(id: bigint): Tag {
    return new Tag(id);
  }
}

export function fill(out: Tag[], upto: bigint): void {
  let n = 0n;
  while (n < upto) {
    out.push(Tag.new(n));
    n = checkedAdd(n, 1n, 'u64');
  }
}

export function howMany(upto: bigint): number {
  let found: Tag[] = [];
  try {
    fill(found, upto);
    return found.length;
  } finally {
    dropOwned(found);
  }
}

export function run(): number {
  return howMany(3n);
}

