// MIRRORS: ankurah/a_local_typed_inside_a_macro_argument/src/input.rs
import { Struct, dropOwned, tracing, checkedAdd } from '@ankurah/base';

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

export function logged(): number {
  let kept: Tag[] = [];
  try {
    tracing.info(`kept ${(() => {
      kept.push(Tag.new(1n));
      return kept.length;
    })()}`);
    return kept.length;
  } finally {
    dropOwned(kept);
  }
}

export function printed(): number {
  let held: Tag[] = [];
  try {
    console.log(`${(() => {
      held.push(Tag.new(2n));
      return held.length;
    })()}`);
    return held.length;
  } finally {
    dropOwned(held);
  }
}

export function run(): number {
  return checkedAdd(logged(), printed(), 'usize');
}

