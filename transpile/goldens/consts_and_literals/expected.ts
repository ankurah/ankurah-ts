// MIRRORS: ankurah/consts_and_literals/src/input.rs
import { Struct, checkedAdd } from '@ankurah/base';

export class Rec extends Struct {
  readonly first: number;
  readonly second: string;
  readonly third: boolean;

  constructor(first: number, second: string, third: boolean) {
    super();
    this.first = first;
    this.second = second;
    this.third = third;
  }

  static make(a: number, b: string, c: boolean): Rec {
    return new Rec(a, b, c);
  }

  tag(): number {
    if (this.third) {
      return TAG_STRING;
    } else {
      return TAG_NULL;
    }
  }
}

export class Point extends Struct {
  x: number;
  readonly y: string;

  constructor(x: number, y: string) {
    super();
    this.x = x;
    this.y = y;
  }

  clone(): Point {
    return new Point(this.x, this.y);
  }
}

export function word(index: number): string {
  return WORDS[index];
}

export function collection(): string {
  return SYSTEM_COLLECTION;
}

export function shifted(): bigint {
  return SHIFT;
}

export function movedOrigin(): number {
  let first = ORIGIN();
  try {
    first.x = 9;
    const second = ORIGIN();
    try {
      return checkedAdd(first.x, second.x, 'u32');
    } finally {
      second.drop();
    }
  } finally {
    first.drop();
  }
}

export function bump(): number {
  return (() => { const _v = COUNTER; COUNTER += 1; return _v; })();
}

export function arm(ready: boolean): boolean {
  READY = ready;
  return READY;
}

export function radix(n: number): number {
  if (n === BASE) {
    return 1;
  } else if (n === 0) {
    return 2;
  } else {
    return 3;
  }
}

export const TAG_NULL: number = 0;

export const TAG_STRING: number = 4;

export const WORDS: string[] = ['ack', 'alabama', 'alanine'];

export const SYSTEM_COLLECTION: string = '_ankurah_system';

const SHIFT: bigint = BigInt.asUintN(64, (1n << 40n));

export function ORIGIN(): Point {
  return new Point(0, '');
}

export let COUNTER: number = 0;

export let READY: boolean = false;

export const FLOOR: bigint = -9007199254740991n;

export const BASE: number = 36;

