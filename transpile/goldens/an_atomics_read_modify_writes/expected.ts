// MIRRORS: ankurah/an_atomics_read_modify_writes/src/input.rs
import { Struct } from '@ankurah/base';

export class Counter extends Struct {
  bits: number;

  constructor(bits: number) {
    super();
    this.bits = bits;
  }

  static new(): Counter {
    return new Counter(10);
  }

  setLowBit(): number {
    return (() => { const _v = this.bits; this.bits = _v | 1; return _v; })();
  }
}

export function bitwiseOverALocal(): number[] {
  let flags = 12;
  const wasOr = (() => { const _v = flags; flags = _v | 3; return _v; })();
  const wasAnd = (() => { const _v = flags; flags = _v & 6; return _v; })();
  const wasXor = (() => { const _v = flags; flags = _v ^ 5; return _v; })();
  return [wasOr, wasAnd, wasXor, flags];
}

export function boundsOverALocal(): number[] {
  let seen = 5;
  const wasMax = (() => { const _v = seen; if (9 > _v) seen = 9; return _v; })();
  const afterMax = seen;
  const wasMin = (() => { const _v = seen; if (2 < _v) seen = 2; return _v; })();
  return [wasMax, afterMax, wasMin, seen];
}

export function logicOverALocal(): boolean[] {
  let ready = true;
  const wasAnd = (() => { const _v = ready; ready = _v && false; return _v; })();
  const wasOr = (() => { const _v = ready; ready = _v || true; return _v; })();
  const wasXor = (() => { const _v = ready; ready = _v !== true; return _v; })();
  return [wasAnd, wasOr, wasXor, ready];
}

export function throughAField(): number[] {
  const counter = Counter.new();
  try {
    const was = counter.setLowBit();
    return [was, counter.bits];
  } finally {
    counter.drop();
  }
}

