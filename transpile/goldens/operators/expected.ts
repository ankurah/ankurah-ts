// MIRRORS: ankurah/operators/src/input.rs
import { Struct, boolAnd, boolOr } from '@ankurah/base';

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

export class Weight extends Struct {
  readonly label: string;
  readonly grams: bigint;

  constructor(label: string, grams: bigint) {
    super();
    this.label = label;
    this.grams = grams;
  }

  add(rhs: Weight): Weight {
    try {
      try {
        return new Weight(this.label, this.grams + rhs.grams);
      } finally {
        rhs.drop();
      }
    } finally {
      this.drop();
    }
  }
}

export class Left extends Struct {
  readonly grams: bigint;

  constructor(grams: bigint) {
    super();
    this.grams = grams;
  }

  add(rhs: Right): bigint {
    return this.grams + rhs.grams;
  }
}

export class Right extends Struct {
  readonly grams: bigint;

  constructor(grams: bigint) {
    super();
    this.grams = grams;
  }
}

export class Parcel extends Struct {
  readonly grams: bigint;

  constructor(grams: bigint) {
    super();
    this.grams = grams;
  }

  add(rhs: Right): bigint {
    try {
      try {
        return this.grams + rhs.grams;
      } finally {
        rhs.drop();
      }
    } finally {
      this.drop();
    }
  }
}

export class Boxed<T> extends Struct {
  readonly value: T;

  constructor(value: T) {
    super();
    this.value = value;
  }

  add(rhs: Boxed<T>): Boxed<T> {
    try {
      return rhs;
    } finally {
      this.drop();
    }
  }
}

export class Charge extends Struct {
  readonly amount: number;

  constructor(amount: number) {
    super();
    this.amount = amount;
  }

  neg(): Charge {
    try {
      return new Charge(-this.amount);
    } finally {
      this.drop();
    }
  }

  not(): Charge {
    try {
      return new Charge((~this.amount | 0));
    } finally {
      this.drop();
    }
  }

  index(_at: number): number {
    return this.amount;
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
  return bits ^ (BigInt.asUintN(64, (1n << 63n)));
}

export function bigger(a: number, b: number): boolean {
  return a > b;
}

export function combined(a: Weight, b: Weight): Weight {
  return a.add(b);
}

export function heavier(a: Weight, b: Weight): boolean {
  const total = a.add(b);
  try {
    return total.grams > 100n;
  } finally {
    total.drop();
  }
}

export function borrowedSum(a: Left, b: Right): bigint {
  return a.add(b);
}

export function laterLocal(parcel: Parcel): bigint {
  const right = new Right(2n);
  return parcel.add(right);
}

export function genericSum(a: Boxed<bigint>, b: Boxed<bigint>): bigint {
  const result = a.add(b);
  try {
    return result.value;
  } finally {
    result.drop();
  }
}

export function eagerAnd(flag: boolean, seen: number[]): boolean {
  return boolAnd(flag, note(seen));
}

export function eagerOr(flag: boolean, seen: number[]): boolean {
  return boolOr(flag, note(seen));
}

function note(seen: number[]): boolean {
  seen.push(1);
  return true;
}

export function toU64(f: number): bigint {
  return (($v) => $v < 0n ? 0n : $v > 18446744073709551615n ? 18446744073709551615n : $v)(BigInt(Math.min(Math.max(Math.trunc(f) || 0, 0), 18446744073709551615)));
}

export function toI64(f: number): bigint {
  return (($v) => $v < -9223372036854775808n ? -9223372036854775808n : $v > 9223372036854775807n ? 9223372036854775807n : $v)(BigInt(Math.min(Math.max(Math.trunc(f) || 0, -9223372036854775808), 9223372036854775807)));
}

export function toF32(v: bigint): number {
  return Math.fround(Number(v));
}

export function shiftAssign32(value: number): number {
  value = ((value << 31) >>> 0);
  return value;
}

export function shiftAssign8(value: number): number {
  value = ((value << 7) & 0xff);
  return value;
}

export function shift64(value: bigint): bigint {
  return BigInt.asUintN(64, (value << 1n));
}

export function shifts(a: number, b: number, c: bigint): [number, number, bigint] {
  return [((a << 31) >>> 0), ((b << 4) & 0xff), BigInt.asUintN(64, (c << 40n))];
}

export function chargeNegated(c: Charge): Charge {
  return c.neg();
}

export function complemented(c: Charge): Charge {
  return c.not();
}

export function indexed(c: Charge): number {
  return c.index(0);
}

