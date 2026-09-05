// MIRRORS: ankurah/condition_chain_temporaries/src/input.rs
import { Struct, Mutex, checkedAdd } from '@ankurah/base';

export class Reading extends Struct {
  readonly level: number;

  constructor(level: number) {
    super();
    this.level = level;
  }
}

export class Meter extends Struct {
  readonly floor: Mutex<number>;

  constructor(floor: Mutex<number>) {
    super();
    this.floor = floor;
  }

  band(level: number): number {
    let _c1;
    const _t0 = reading(level);
    try {
      _c1 = _t0.level > 10;
    } finally {
      _t0.drop();
    }
    if (_c1) {
      return 3;
    } else {
      let _c3;
      const _t2 = reading(level);
      try {
        _c3 = _t2.level > 5;
      } finally {
        _t2.drop();
      }
      if (_c3) {
        return 2;
      } else {
        let _c5;
        const _t4 = this.floor.lock();
        try {
          _c5 = _t4.value > level;
        } finally {
          _t4.drop();
        }
        if (_c5) {
          return 1;
        } else {
          return 0;
        }
      }
    }
  }

  climb(): number {
    let level = 0;
    for (;;) {
      let _c1;
      const _t0 = reading(level);
      try {
        _c1 = _t0.level < 3;
      } finally {
        _t0.drop();
      }
      if (!_c1) break;
      level = checkedAdd(level, 1, 'usize');
    }
    return level;
  }
}

export function reading(level: number): Reading {
  return new Reading(level);
}

