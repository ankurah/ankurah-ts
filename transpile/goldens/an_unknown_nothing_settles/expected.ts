// MIRRORS: ankurah/an_unknown_nothing_settles/src/input.rs
import { Struct, dropOwned } from '@ankurah/base';

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

export function counted(n: number): number {
  let held = [];
  if (n > 0) {
    held.splice(0, 1)[0];
  }
  return held.length;
}

export function filled(): number {
  let held = [];
  try {
    held.push(Tag.new(1n));
    return held.length;
  } finally {
    dropOwned(held);
  }
}

