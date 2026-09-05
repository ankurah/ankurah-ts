// MIRRORS: ankurah/nested_variant_test/src/input.rs
import { Struct, Enum, HashMap, HashSet, keyHash } from '@ankurah/base';

export class Id extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Pair extends Struct {
  readonly a: string;
  readonly b: string;

  constructor(a: string, b: string) {
    super();
    this.a = a;
    this.b = b;
  }

  equals(other: Pair): boolean {
    if (this.a !== other.a) return false;
    if (this.b !== other.b) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.a), keyHash(this.b)].map((p) => p.length + ':' + p).join('');
  }
}

export type StatusV = {
  Requested: { _0: Id; _1: number };
  Established: { _0: Id; _1: number };
  Idle: {};
};

export class Status extends Enum<StatusV> {
}

export type WrapV = {
  Inner: { _0: Status };
  Other: {};
};

export class Wrap extends Enum<WrapV> {
}

export function isRequested(s: Status | null): boolean {
  if (s != null && (s.is('Requested'))) {
    return true;
  } else {
    return false;
  }
}

export function wrapsRequested(w: Wrap): boolean {
  if (w.is('Inner') && (w.value._0.is('Requested'))) {
    return true;
  } else {
    return false;
  }
}

export function isAnything(s: Status | null): boolean {
  if (s != null) {
    return true;
  } else {
    return false;
  }
}

