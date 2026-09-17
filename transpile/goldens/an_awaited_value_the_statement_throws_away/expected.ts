// MIRRORS: ankurah/an_awaited_value_the_statement_throws_away/src/input.rs
import { Struct } from '@ankurah/base';

export class Held extends Struct {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  static new(n: bigint): Held {
    return new Held(n);
  }
}

export async function makeHeld(n: bigint): Promise<Held> {
  return Held.new(n);
}

export async function discardsWhatItAwaited(): Promise<bigint> {
  (await makeHeld(1n)).drop();
  const _t0 = (await makeHeld(2n));
  try {
    return _t0.n;
  } finally {
    _t0.drop();
  }
}

