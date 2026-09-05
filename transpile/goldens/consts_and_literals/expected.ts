// MIRRORS: ankurah/consts_and_literals/src/input.rs
import { Struct } from '@ankurah/base';

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

export function word(index: number): string {
  return WORDS[index];
}

export function collection(): string {
  return SYSTEM_COLLECTION;
}

export function shifted(): bigint {
  return SHIFT;
}

export const TAG_NULL: number = 0;

export const TAG_STRING: number = 4;

export const WORDS: string[] = ['ack', 'alabama', 'alanine'];

export const SYSTEM_COLLECTION: string = '_ankurah_system';

const SHIFT: bigint = BigInt.asUintN(64, (1n << 40n));

