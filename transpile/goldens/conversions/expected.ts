// MIRRORS: ankurah/conversions/src/input.rs
import { Struct } from '@ankurah/base';

export class Tag extends Struct {
  readonly label: number;

  constructor(label: number) {
    super();
    this.label = label;
  }
}

export class Name extends Struct {
  readonly text: string;

  constructor(text: string) {
    super();
    this.text = text;
  }

  static fromTag(tag: Tag): Name {
    try {
      return new Name(tag.label.toString());
    } finally {
      tag.drop();
    }
  }
}

export class Sizes extends Struct {
  readonly _0: string;

  constructor(_0: string) {
    super();
    this._0 = _0;
  }

  static fromVecU32(v: number[]): Sizes {
    return new Sizes(`u${v.length}`);
  }

  static fromVecI32(v: number[]): Sizes {
    return new Sizes(`i${v.length}`);
  }

  static fromU32(v: number[]): Sizes {
    return new Sizes(`s${v.length}`);
  }
}

export function named(tag: Tag): Name {
  return Name.fromTag(tag);
}

export function fromCall(tag: Tag): Name {
  return Name.fromTag(tag);
}

export function owned(raw: string): string {
  return raw;
}

export function widen(n: number): bigint {
  return BigInt(n);
}

export function narrow(n: bigint): number {
  return Number(BigInt.asUintN(32, n));
}

export function truncate(f: number): number {
  return Math.min(Math.max(Math.trunc(f) || 0, -2147483648), 2147483647);
}

