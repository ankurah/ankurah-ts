// MIRRORS: ankurah/a_cloned_generic_payload/src/input.rs
import { Struct, Drop, dropOwned, derivedClone, checkedAdd, HashMap } from '@ankurah/base';

export class Holder extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }

  clone(): Holder {
    return new Holder(this.n);
  }
}

export class Bag<V extends Clone> extends Struct {
  readonly inner: HashMap<bigint, V>;

  constructor(inner: HashMap<bigint, V>) {
    super();
    this.inner = inner;
  }

  get(k: bigint): V | null {
    const _m0 = this.inner.get(k);
    return (_m0 != null ? derivedClone(_m0) : null);
  }

  all(): V[] {
    return [...this.inner.values()].map((e) => derivedClone(e));
  }
}

export function takeOne(bag: Bag<Holder>, k: bigint): bigint {
  const _v = bag.get(k);
  if (_v != null) {
    const held = _v;
    try {
      return held.n;
    } finally {
      held.drop();
    }
  } else {
    return 0n;
  }
}

export function takeAll(bag: Bag<Holder>): bigint {
  let total = 0n;
  const _seq0 = bag.all();
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const held = _seq0[_at1++];
      try {
        total = checkedAdd(total, held.n, 'u64');
      } finally {
        held.drop();
      }
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  return total;
}

