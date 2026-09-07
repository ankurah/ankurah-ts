// MIRRORS: ankurah/what_the_port_evaluates/src/input.rs
import { Struct, Drop } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export class Inner extends Struct {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }
}

export class Handle extends Struct {
  inner: Inner;

  constructor(inner: Inner) {
    super();
    this.inner = inner;
  }

  static new(n: bigint): Handle {
    return new Handle(new Inner(n));
  }

  deref(): Inner {
    return this.inner;
  }
}

export class Event extends Struct {
  readonly t: Token;
  readonly n: bigint;

  constructor(t: Token, n: bigint) {
    super();
    this.t = t;
    this.n = n;
  }
}

export class Reordered extends Struct {
  readonly token: Token;
  readonly n: bigint;

  constructor(token: Token, n: bigint) {
    super();
    this.token = token;
    this.n = n;
  }
}

export function mk(n: bigint): Token {
  return new Token(n);
}

export function throughADeref(token: Token, handle: Handle): Event {
  let _moved0 = false;
  try {
    try {
      const _b1 = handle.deref().n;
      _moved0 = true;
      return new Event(token, _b1);
    } finally {
      handle.drop();
    }
  } finally {
    if (!_moved0) token.drop();
  }
}

export function reordered(token: Token, value: bigint | null): Reordered {
  let _moved0 = false;
  try {
    const _b1 = (value ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
    _moved0 = true;
    return new Reordered(token, _b1);
  } finally {
    if (!_moved0) token.drop();
  }
}

export function reassigned(low: Token, again: boolean, value: bigint | null): bigint {
  let _moved0 = false;
  let low_1 = low;
  try {
    if (again) {
      const _a1 = mk(2n);
      low_1.drop();
      low_1 = _a1;
    }
    const _b2 = (value ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
    _moved0 = true;
    const built = new Reordered(low_1, _b2);
    try {
      return built.n;
    } finally {
      built.drop();
    }
  } finally {
    if (!_moved0) low_1.drop();
  }
}

