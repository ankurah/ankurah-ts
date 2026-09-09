// MIRRORS: ankurah/a_callable_reached_through_what_holds_it/src/input.rs
import { Struct, Arc, OwnedClosure, invoke, invokeRef, Invocable, dropOwned, checkedAdd } from '@ankurah/base';

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

export class Holder extends Struct {
  readonly call: Arc<Invocable<[bigint], bigint>>;

  constructor(call: Arc<Invocable<[bigint], bigint>>) {
    super();
    this.call = call;
  }

  static new(call: Arc<Invocable<[bigint], bigint>>): Holder {
    return new Holder(call);
  }

  run(n: bigint): bigint {
    return invokeRef(this.call.value, n);
  }
}

export function throughAHolder(): bigint {
  const tag = Tag.new(3n);
  const holder = Holder.new(Arc.new(new OwnedClosure([tag], (n: bigint) => checkedAdd(n, tag.n, 'u64'))));
  try {
    return holder.run(4n);
  } finally {
    holder.drop();
  }
}

export function makeBox(): Invocable<[bigint], bigint> {
  const tag = Tag.new(2n);
  return new OwnedClosure([tag], (n: bigint) => checkedAdd(n, tag.n, 'u64'));
}

export function fromAnExpression(): bigint {
  const _t0 = makeBox();
  try {
    return invokeRef(_t0, 5n);
  } finally {
    dropOwned(_t0);
  }
}

export function makeOnce(): Invocable<[], bigint> {
  const tag = Tag.new(9n);
  return new OwnedClosure([tag], () => tag.n);
}

export function consumedOnce(): bigint {
  return invoke(makeOnce());
}

