// MIRRORS: ankurah/an_atomics_operands_run_once/src/input.rs
import { Struct, Result, checkedAdd, wrappingAdd } from '@ankurah/base';

export class Ticker extends Struct {
  calls: number;

  constructor(calls: number) {
    super();
    this.calls = calls;
  }

  static new(): Ticker {
    return new Ticker(0);
  }

  next(): number {
    return checkedAdd((() => { const _n = 1; const _v = this.calls; this.calls = wrappingAdd(_v, _n, 'usize'); return _v; })(), 1, 'usize');
  }

  flag(): boolean {
    return this.next() > 0;
  }
}

export function boundsReadTheirOperandOnce(): number[] {
  const tick = Ticker.new();
  try {
    let seen = 0;
    const wasMax = (() => { const _n = tick.next(); const _v = seen; if (_n > _v) seen = _n; return _v; })();
    const afterMax = seen;
    const wasMin = (() => { const _n = tick.next(); const _v = seen; if (_n < _v) seen = _n; return _v; })();
    return [wasMax, afterMax, wasMin, seen, tick.calls];
  } finally {
    tick.drop();
  }
}

export function logicReadsItsOperandOnce(): boolean[] {
  const tick = Ticker.new();
  try {
    let off = false;
    let on = true;
    const wasAnd = (() => { const _n = tick.flag(); const _v = off; off = _v && _n; return _v; })();
    const wasOr = (() => { const _n = tick.flag(); const _v = on; on = _v || _n; return _v; })();
    return [wasAnd, off, wasOr, on, tick.calls === 2];
  } finally {
    tick.drop();
  }
}

export function compareExchangeReadsItsNewValueOnce(): boolean[] {
  const tick = Ticker.new();
  try {
    let value = 1;
    const _t0 = (() => { const _want = 2; const _next = tick.next(); const _v = value; if (_v === _want) { value = _next; return Result.Ok(_v); } return Result.Err(_v); })();
    try {
      const missed = _t0.isOk();
      const _t1 = (() => { const _want = 1; const _next = tick.next(); const _v = value; if (_v === _want) { value = _next; return Result.Ok(_v); } return Result.Err(_v); })();
      try {
        const took = _t1.isOk();
        return [missed, took, value === 2, tick.calls === 2];
      } finally {
        _t1.drop();
      }
    } finally {
      _t0.drop();
    }
  } finally {
    tick.drop();
  }
}

export function aHighBitStaysUnsigned(): number[] {
  let flags = 1;
  const wasOr = (() => { const _n = 2147483648; const _v = flags; flags = ((_v | _n) >>> 0); return _v; })();
  const afterOr = flags;
  const wasAnd = (() => { const _n = 4294901760; const _v = flags; flags = ((_v & _n) >>> 0); return _v; })();
  return [wasOr, afterOr, wasAnd, flags];
}

