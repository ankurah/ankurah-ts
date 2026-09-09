// MIRRORS: ankurah/a_callback_held_in_a_field/src/input.rs
import { Struct, OwnedClosure, invokeRef, Invocable, checkedAdd } from '@ankurah/base';

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

export class Holder extends Struct {
  readonly compute: Invocable<[], bigint>;

  constructor(compute: Invocable<[], bigint>) {
    super();
    this.compute = compute;
  }

  static new(compute: Invocable<[], bigint>): Holder {
    return new Holder(compute);
  }

  read(): bigint {
    return invokeRef(this.compute);
  }
}

export function overACapture(): bigint {
  const tag = Tag.new(6n);
  const holder = Holder.new(new OwnedClosure([tag], () => tag.id));
  try {
    return checkedAdd(holder.read(), holder.read(), 'u64');
  } finally {
    holder.drop();
  }
}

