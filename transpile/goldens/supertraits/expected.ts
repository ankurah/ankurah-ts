// MIRRORS: ankurah/supertraits/src/input.rs
import { Struct, checkedAdd, checkedMul } from '@ankurah/base';
import { Buried } from './input/nested';

export class One extends Struct implements Tell, Super {

  tell(): number {
    return 1;
  }
}

export class Two extends Struct implements Buried<number>, Deep {

  buried(): number {
    return 7;
  }

  deep(): number {
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

export interface Deep extends Buried<number> {
  deep(): number;
}

export function ask<T extends Super>(t: T): number {
  return t.tell();
}

export function dig<T extends Deep>(t: T): number {
  return checkedAdd(t.buried(), t.deep(), 'u32');
}

