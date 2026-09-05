// MIRRORS: ankurah/option_match_ownership/src/input.rs
import { Struct, checkedAdd } from '@ankurah/base';

export class Token extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  static new(n: number): Token {
    return new Token(n);
  }
}

export function consume(token: Token): number {
  try {
    return token.n;
  } finally {
    token.drop();
  }
}

export function read(slot: Token | null): number {
  if (slot != null) {
    const token = slot;
    try {
      return checkedAdd(token.n, 1, 'usize');
    } finally {
      token.drop();
    }
  } else {
    return 0;
  }
}

export function handOn(slot: Token | null): number {
  if (slot != null) {
    const token = slot;
    return consume(token);
  } else {
    return 0;
  }
}

export function either(slot: Token | null, keep: boolean): number {
  if (slot != null) {
    const token = slot;
    let _moved0 = false;
    try {
      if (keep) {
        return checkedAdd(token.n, 100, 'usize');
      } else {
        _moved0 = true;
        return consume(token);
      }
    } finally {
      if (!_moved0) token.drop();
    }
  } else {
    return 0;
  }
}

export function peek(slot: Token | null): number {
  if (slot != null) {
    const token = slot;
    return token.n;
  } else {
    return 0;
  }
}

