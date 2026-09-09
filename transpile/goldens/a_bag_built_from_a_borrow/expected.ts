// MIRRORS: ankurah/a_bag_built_from_a_borrow/src/input.rs
import { Struct, dropOwned, checkedAdd } from '@ankurah/base';

export class Tag extends Struct {
  readonly text: string;

  constructor(text: string) {
    super();
    this.text = text;
  }

  static new(text: string): Tag {
    return new Tag(text);
  }
}

export class Bag<T> extends Struct {
  items: T[];

  constructor(items: T[]) {
    super();
    this.items = items;
  }

  static new<T, I extends Iterable<T>>(items: I): Bag<T> {
    let held = [];
    for (const item of items) {
      held.push(item);
    }
    return new Bag(held);
  }

  drain(): T[] {
    let out = [];
    for (;;) {
      const _v = this.items.pop();
      if (!(_v != null)) {
        break;
      }
      const item = _v;
      out.push(item);
    }
    return out;
  }
}

export function widthsOf(held: Tag[]): number {
  let bag = Bag.new(held);
  try {
    const taken = bag.drain();
    let total = 0;
    for (const tag of taken) {
      total = checkedAdd(total, tag.text.length, 'usize');
    }
    return total;
  } finally {
    bag.drop();
  }
}

export function run(): number {
  const tags = [Tag.new('aa'), Tag.new('b')];
  try {
    return widthsOf(tags);
  } finally {
    dropOwned(tags);
  }
}

