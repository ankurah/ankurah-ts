// MIRRORS: ankurah/a_calculation_typed_inside_its_block/src/input.rs
import { Struct, OwnedClosure, invokeRef, Invocable, checkedMul } from '@ankurah/base';

export class Source extends Struct {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  static new(n: bigint): Source {
    return new Source(n);
  }

  reader(): Reader {
    return new Reader(this.n);
  }
}

export class Reader extends Struct {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  get(): bigint {
    return this.n;
  }
}

export class Calc<T> extends Struct {
  readonly compute: Invocable<[], T>;

  constructor(compute: Invocable<[], T>) {
    super();
    this.compute = compute;
  }

  static make<T>(compute: Invocable<[], T>): Calc<T> {
    return new Calc(compute);
  }

  read(): T {
    return invokeRef(this.compute);
  }
}

export function doubled(): bigint {
  const source = Source.new(3n);
  try {
    const calc = Calc.make((() => {
      const reader = source.reader();
      return new OwnedClosure([reader], () => checkedMul(reader.get(), 2n, 'u64'));
    })());
    try {
      return calc.read();
    } finally {
      calc.drop();
    }
  } finally {
    source.drop();
  }
}

