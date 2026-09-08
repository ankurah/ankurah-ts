// MIRRORS: ankurah/the_callee_bound_types_the_closure/src/input.rs
import { Struct, Drop, invokeRef, Invocable, dropOwned } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function apply(f: Invocable<[Token], bigint>): bigint {
  try {
    return invokeRef(f, new Token(1n));
  } finally {
    dropOwned(f);
  }
}

export function genericApply<T>(v: T, f: Invocable<[T], bigint>): bigint {
  try {
    return invokeRef(f, v);
  } finally {
    dropOwned(f);
  }
}

export function parenthesised(): bigint {
  return apply(((held) => {
    try {
      return held.n;
    } finally {
      held.drop();
    }
  }));
}

export function plain(): bigint {
  return apply((held) => {
    try {
      return held.n;
    } finally {
      held.drop();
    }
  });
}

export function fromASibling(t: Token): bigint {
  return genericApply(t, (x) => {
    try {
      return x.n;
    } finally {
      x.drop();
    }
  });
}

