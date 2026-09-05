// MIRRORS: ankurah/condition_guard_field/src/input.rs
import { Struct, Mutex, checkedAdd, checkedSub } from '@ankurah/base';

export class Slot extends Struct {
  n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Cell extends Struct {
  readonly slot: Mutex<Slot>;

  constructor(slot: Mutex<Slot>) {
    super();
    this.slot = slot;
  }

  clear(): boolean {
    let _c1;
    const _t0 = this.slot.lock();
    try {
      _c1 = _t0.value.n > 0;
    } finally {
      _t0.drop();
    }
    if (_c1) {
      let guard = this.slot.lock();
      try {
        guard.value.n = 0;
        return true;
      } finally {
        guard.drop();
      }
    }
    return false;
  }

  drain(): number {
    let turns = 0;
    for (;;) {
      let _c1;
      const _t0 = this.slot.lock();
      try {
        _c1 = _t0.value.n > 0;
      } finally {
        _t0.drop();
      }
      if (!_c1) break;
      let guard = this.slot.lock();
      try {
        guard.value.n = checkedSub(guard.value.n, 1, 'usize');
        turns = checkedAdd(turns, 1, 'usize');
      } finally {
        guard.drop();
      }
    }
    return turns;
  }
}

