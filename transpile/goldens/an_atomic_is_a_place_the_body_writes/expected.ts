// MIRRORS: ankurah/an_atomic_is_a_place_the_body_writes/src/input.rs
import { Struct, Result, wrappingAdd } from '@ankurah/base';

export class Counter extends Struct {
  hits: number;
  alive: boolean;

  constructor(hits: number, alive: boolean) {
    super();
    this.hits = hits;
    this.alive = alive;
  }

  static new(): Counter {
    return new Counter(0, true);
  }

  bump(): number {
    return (() => { const _v = this.hits; this.hits = wrappingAdd(this.hits, 1, 'usize'); return _v; })();
  }

  read(): number {
    return this.hits;
  }

  close(): boolean {
    return (() => { const _v = this.alive; this.alive = false; return _v; })();
  }

  claim(): boolean {
    const _t0 = (() => { const _v = this.alive; if (_v === true) { this.alive = false; return Result.Ok(_v); } return Result.Err(_v); })();
    try {
      return _t0.isOk();
    } finally {
      _t0.drop();
    }
  }
}

export function counted(): number {
  const counter = Counter.new();
  try {
    counter.bump();
    counter.bump();
    return counter.read();
  } finally {
    counter.drop();
  }
}

export function closed(): boolean {
  const counter = Counter.new();
  try {
    return counter.close();
  } finally {
    counter.drop();
  }
}

export function claimedTwice(): boolean {
  const counter = Counter.new();
  try {
    const first = counter.claim();
    const second = counter.claim();
    return first && !second;
  } finally {
    counter.drop();
  }
}

export function stored(): number {
  let held = 0;
  held = 3;
  return held;
}

