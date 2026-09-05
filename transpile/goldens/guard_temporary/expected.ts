// MIRRORS: ankurah/guard_temporary/src/input.rs
import { Struct, Mutex, checkedAdd } from '@ankurah/base';

export class Counter extends Struct {
  readonly value: Mutex<number>;

  constructor(value: Mutex<number>) {
    super();
    this.value = value;
  }

  read(): number {
    const _t0 = this.value.lock();
    try {
      const seen = _t0.value;
      _t0.drop();
      return checkedAdd(seen, 1, 'usize');
    } finally {
      _t0.drop();
    }
  }

  bump(): number {
    let guard = this.value.lock();
    try {
      guard.value += 1;
      return guard.value;
    } finally {
      guard.drop();
    }
  }
}

