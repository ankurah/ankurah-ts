// MIRRORS: ankurah/match_guard_temporary/src/input.rs
import { Struct, checkedMul } from '@ankurah/base';

export class Reading extends Struct {
  readonly limit: number;

  constructor(limit: number) {
    super();
    this.limit = limit;
  }
}

export function limitOf(scale: number): Reading {
  return new Reading(checkedMul(scale, 2, 'usize'));
}

export function classify(value: number, scale: number): number {
  if (value === 0) {
    return 0;
  }
  {
    const n = value;
    let _c1;
    const _t0 = limitOf(scale);
    try {
      _c1 = _t0.limit > n;
    } finally {
      _t0.drop();
    }
    if (_c1) {
      return 1;
    }
  }
  {
    return 2;
  }
}

export function banded(value: number, scale: number): number {
  {
    const n = value;
    let _c1;
    const _t0 = limitOf(scale);
    try {
      _c1 = _t0.limit > n;
    } finally {
      _t0.drop();
    }
    if (_c1) {
      return 1;
    }
  }
  {
    const n = value;
    let _c3;
    const _t2 = limitOf(checkedMul(scale, 4, 'usize'));
    try {
      _c3 = _t2.limit > n;
    } finally {
      _t2.drop();
    }
    if (_c3) {
      return 2;
    }
  }
  {
    return 3;
  }
}

