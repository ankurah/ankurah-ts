// MIRRORS: ankurah/a_cloned_sequence_owns_its_copies/src/input.rs
import { Struct, Drop, dropOwned, checkedAdd, HashMap } from '@ankurah/base';

export class Listener extends Drop {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }

  protected override onDrop(): void {

  }

  clone(): Listener {
    return new Listener(this.id);
  }
}

export function totalOfEveryListener(): bigint {
  let listeners = new HashMap<bigint, Listener>();
  try {
    listeners.set(1n, new Listener(1n));
    listeners.set(2n, new Listener(2n));
    const taken = [...listeners.values()].map((e) => e.clone());
    let total = 0n;
    const _seq0 = taken;
    let _at1 = 0;
    try {
      while (_at1 < _seq0.length) {
        const listener = _seq0[_at1++];
        try {
          total = checkedAdd(total, listener.id, 'u64');
        } finally {
          listener.drop();
        }
      }
    } finally {
      dropOwned(_seq0.slice(_at1));
    }
    return total;
  } finally {
    dropOwned(listeners);
  }
}

