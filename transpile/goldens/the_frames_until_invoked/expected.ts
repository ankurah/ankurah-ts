// MIRRORS: ankurah/the_frames_until_invoked/src/input.rs
import { Struct, Drop, dropOwned, checkedAdd } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export class Box3 extends Struct {
  readonly a: Token | null;
  readonly k: Token;
  readonly c: bigint;

  constructor(a: Token | null, k: Token, c: bigint) {
    super();
    this.a = a;
    this.k = k;
    this.c = c;
  }
}

export class Box4 extends Struct {
  readonly a: Token;
  readonly b: Token;
  readonly c: bigint;

  constructor(a: Token, b: Token, c: bigint) {
    super();
    this.a = a;
    this.b = b;
    this.c = c;
  }
}

export function mk(n: bigint): Token {
  return new Token(n);
}

export function take2(a: Token, b: bigint): bigint {
  try {
    return checkedAdd(a.n, b, 'i64');
  } finally {
    a.drop();
  }
}

export function afterAQuestion(t: Token, o: bigint | null): bigint | null {
  let _moved0 = false;
  try {
    const _r1 = o;
    if (_r1 == null) return null;
    _moved0 = true;
    return take2(t, _r1);
  } finally {
    if (!_moved0) t.drop();
  }
}

export function inATuple(token: Token, o: bigint | null): [Token, bigint] {
  let _moved0 = false;
  try {
    const _b1 = (o ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
    _moved0 = true;
    return [token, _b1];
  } finally {
    if (!_moved0) token.drop();
  }
}

export function nestedInAnOperand(x: Token, k: Token, o: bigint | null): Box3 {
  let _moved0 = false;
  let _moved1 = false;
  try {
    try {
      const _b2 = (o ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
      _moved1 = true;
      _moved0 = true;
      return new Box3(x, k, _b2);
    } finally {
      if (!_moved1) k.drop();
    }
  } finally {
    if (!_moved0) x.drop();
  }
}

export function onlyTemporaries(o: bigint | null): Box4 {
  let _moved1 = false;
  const _b0 = mk(1n);
  try {
    let _moved3 = false;
    const _b2 = mk(2n);
    try {
      const _b4 = (o ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
      _moved1 = true;
      _moved3 = true;
      return new Box4(_b0, _b2, _b4);
    } finally {
      if (!_moved3) dropOwned(_b2);
    }
  } finally {
    if (!_moved1) dropOwned(_b0);
  }
}

