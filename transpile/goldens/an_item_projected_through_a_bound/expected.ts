// MIRRORS: ankurah/an_item_projected_through_a_bound/src/input.rs
import { Struct, checkedAdd } from '@ankurah/base';

export class Tag extends Struct {
  readonly text: string;

  constructor(text: string) {
    super();
    this.text = text;
  }
}

export class Holder<F> extends Struct {
  items: F[];

  constructor(items: F[]) {
    super();
    this.items = items;
  }

  static new<F, I extends Iterable<F>>(items: I): Holder<F> {
    let held = [];
    for (const item of items) {
      held.push(item);
    }
    return new Holder(held);
  }

  take(): F {
    return this.items.splice(0, 1)[0];
  }

  count(): number {
    return this.items.length;
  }
}

export function takeOne(tags: Tag[]): number {
  let holder = Holder.new(tags);
  try {
    const taken = holder.take();
    try {
      const left = holder.count();
      return checkedAdd(left, taken.text.length, 'usize');
    } finally {
      taken.drop();
    }
  } finally {
    holder.drop();
  }
}

