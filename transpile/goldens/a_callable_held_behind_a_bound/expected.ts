// MIRRORS: ankurah/a_callable_held_behind_a_bound/src/input.rs
import { Struct, Arc, invokeRef, Invocable, checkedMul } from '@ankurah/base';

export class Doubler<F extends Invocable<[bigint], bigint>> extends Struct {
  readonly inner: Arc<F>;

  constructor(inner: Arc<F>) {
    super();
    this.inner = inner;
  }

  static new<F>(inner: F): Doubler<F> {
    return new Doubler(Arc.new(inner));
  }

  apply(n: bigint): bigint {
    return invokeRef(this.inner.value, n);
  }
}

export function throughABareName(f: Arc<Invocable<[bigint], bigint>>): bigint {
  try {
    return invokeRef(f.value, 4n);
  } finally {
    f.drop();
  }
}

export function heldBehindABound(): bigint {
  const doubler = Doubler.new((n) => checkedMul(n, 2n, 'u64'));
  try {
    return doubler.apply(21n);
  } finally {
    doubler.drop();
  }
}

