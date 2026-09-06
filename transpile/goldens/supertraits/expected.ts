// MIRRORS: ankurah/supertraits/src/input.rs
import { Struct, checkedMul } from '@ankurah/base';

export class One extends Struct implements Tell, Super {

  tell(): number {
    return 1;
  }
}

export interface Tell {
  tell(): number;
}

export interface Super extends Tell {
}

export interface Loud extends Tell {}

export abstract class Loud {
  shout(): number {
    return checkedMul(this.tell(), 2, 'u32');
  }
}

export function ask<T extends Super>(t: T): number {
  return t.tell();
}

