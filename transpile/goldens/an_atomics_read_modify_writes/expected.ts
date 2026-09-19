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
    return (() => { const _n = 1; const _v = this.bits; this.bits = ((_v | _n) >>> 0); return _v; })();
  }
}

export function bitwiseOverALocal(): number[] {
  let flags = 12;
  const wasOr = (() => { const _n = 3; const _v = flags; flags = ((_v | _n) >>> 0); return _v; })();
  const wasAnd = (() => { const _n = 6; const _v = flags; flags = ((_v & _n) >>> 0); return _v; })();
  const wasXor = (() => { const _n = 5; const _v = flags; flags = ((_v ^ _n) >>> 0); return _v; })();
  return [wasOr, wasAnd, wasXor, flags];
}

export function boundsOverALocal(): number[] {
  let seen = 5;
  const wasMax = (() => { const _n = 9; const _v = seen; if (_n > _v) seen = _n; return _v; })();
  const afterMax = seen;
  const wasMin = (() => { const _n = 2; const _v = seen; if (_n < _v) seen = _n; return _v; })();
  return [wasMax, afterMax, wasMin, seen];
}

export function logicOverALocal(): boolean[] {
  let ready = true;
  const wasAnd = (() => { const _n = false; const _v = ready; ready = _v && _n; return _v; })();
  const wasOr = (() => { const _n = true; const _v = ready; ready = _v || _n; return _v; })();
  const wasXor = (() => { const _n = true; const _v = ready; ready = _v !== _n; return _v; })();
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

