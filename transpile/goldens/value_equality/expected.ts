// MIRRORS: ankurah/value_equality/src/input.rs
import { Struct, Enum, valueEquals, HashMap, HashSet } from '@ankurah/base';

export class Tag extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export type KindV = {
  Small: {};
  Large: {};
};

export class Kind extends Enum<KindV> {

  clone(): Kind {
    return new Kind(this.type, { ...this.value });
  }

  equals(other: Kind): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return String(this.type);
  }

  debug(): string {
    return this.match({
      Small: () => 'Small',
      Large: () => 'Large',
    });
  }
}

export function isZero(bytes: Uint8Array): boolean {
  return valueEquals(bytes, new Uint8Array([0, 0, 0, 0]));
}

export function sameMembers(a: HashSet<number>, b: HashSet<number>): boolean {
  return valueEquals(a, b);
}

export function sameKind(l: Kind, r: Kind): boolean {
  return l.equals(r);
}

export function differentTag(l: Tag, r: Tag): boolean {
  return l.name !== r.name;
}

