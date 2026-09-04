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
  return (Math.trunc(f) | 0);
}

