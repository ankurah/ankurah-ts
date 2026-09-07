// MIRRORS: ankurah/lift_without_a_transfer/src/input.rs
import { Struct, dropOwned, unsupported } from '@ankurah/base';

export class Token extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Spill extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  clone(): Spill {
    return new Spill(this.n);
  }
}

export class Rows extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  topK(spill: Spill, k: number): Token[] {
    try {
      const _ = [spill, k];
      return [];
    } finally {
      this.drop();
    }
  }
}

export class Inner extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Handle extends Struct {
  readonly _0: Inner | null;

  constructor(_0: Inner | null) {
    super();
    this._0 = _0;
  }

  make(token: Token, leave: boolean): number {
    let _moved0 = false;
    try {
      if (leave) {
        return 0;
      }
      const _b1 = this.deref().n;
      _moved0 = true;
      const e = new Event(token, _b1);
      try {
        return e.n;
      } finally {
        e.drop();
      }
    } finally {
      if (!_moved0) token.drop();
    }
  }

  deref(): Inner {
    return (this._0 ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
  }
}

export class Event extends Struct {
  readonly token: Token;
  readonly n: number;

  constructor(token: Token, n: number) {
    super();
    this.token = token;
    this.n = n;
  }
}

export function tally<T>(x: T): number {
  const _ = x;
  return 0;
}

export function refusedCallee(rows: Rows, spill: Spill, limit: number | null, leave: boolean): number {
  try {
    const held = rows;
    try {
      if (leave) {
        return 0;
      }
      if (limit != null) {
        const k = limit;
        const _b1 = spill.clone();
        try {
          const _b3 = k;
          return tally(unsupported('`collect` builds whatever its target type names, and the engine could not name the type this one is collected into'));
        } finally {
          dropOwned(_b1);
        }
      } else {
        return 0;
      }
    } finally {
      held.drop();
    }
  } finally {
    spill.drop();
  }
}

export function refusedCalleeUnflagged(tokens: Token[], spill: Spill, limit: number | null, leave: boolean): number {
  try {
    const held = tokens;
    try {
      if (leave) {
        return 0;
      }
      if (limit != null) {
        const k = limit;
        {
          const _ = k;
          const _b1 = [spill.clone()];
          try {
            return unsupported('`collect` into `number` is a `FromIterator` the port has no construction for');
          } finally {
            dropOwned(_b1);
          }
        }
      } else {
        return 0;
      }
    } finally {
      dropOwned(held);
    }
  } finally {
    spill.drop();
  }
}

