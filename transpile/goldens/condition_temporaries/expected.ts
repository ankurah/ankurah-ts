// MIRRORS: ankurah/condition_temporaries/src/input.rs
import { Struct, Mutex } from '@ankurah/base';

export class Counter extends Struct {
  readonly value: Mutex<number>;

  constructor(value: Mutex<number>) {
    super();
    this.value = value;
  }

  startIfIdle(): boolean {
    let _c1;
    const _t0 = this.value.lock();
    try {
      _c1 = _t0.value === 0;
    } finally {
      _t0.drop();
    }
    if (_c1) {
      let guard = this.value.lock();
      try {
        guard.value = 1;
        return true;
      } finally {
        guard.drop();
      }
    }
    return false;
  }

  windDown(): number {
    let turns = 0;
    for (;;) {
      let _c1;
      const _t0 = this.value.lock();
      try {
        _c1 = _t0.value > 0;
      } finally {
        _t0.drop();
      }
      if (!_c1) break;
      let guard = this.value.lock();
      try {
        guard.value -= 1;
        turns += 1;
      } finally {
        guard.drop();
      }
    }
    return turns;
  }
}

