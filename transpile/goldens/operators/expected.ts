// MIRRORS: ankurah/operators/src/input.rs
import { Struct } from '@ankurah/base';

export class Tag extends Struct {
  readonly id: number;

  constructor(id: number) {
    super();
    this.id = id;
  }

  equals(other: Tag): boolean {
    if (this.id !== other.id) return false;
    return true;
  }

  clone(): Tag {
    return new Tag(this.id);
  }
}

export function same(a: Tag, b: Tag): boolean {
  return a.equals(b);
}

export function different(a: Tag, b: Tag): boolean {
  return !a.equals(b);
}

export function halves(n: number): number {
  return Math.trunc(n / 2);
}

export function flipped(bits: number): number {
  return (~bits >>> 0);
}

export function negated(yes: boolean): boolean {
  return !yes;
}

export function shifted(bits: bigint): bigint {
  return bits ^ (1n << 63n);
}

export function bigger(a: number, b: number): boolean {
  return a > b;
}

