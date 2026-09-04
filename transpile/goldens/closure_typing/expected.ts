// MIRRORS: ankurah/closure_typing/src/input.rs
import { Struct } from '@ankurah/base';

export class Reading extends Struct {
  readonly level: number;

  constructor(level: number) {
    super();
    this.level = level;
  }

  doubled(): number {
    return this.level * 2;
  }
}

export function eachDoubled(readings: Reading[]): number[] {
  return [...readings].map((reading) => reading.doubled());
}

export function scaled(readings: Reading[]): number[] {
  return [...readings].map((reading) => reading.level);
}

export function threshold(limit: number): (arg0: number) => boolean {
  return (level) => level > limit;
}

export function counted(readings: Reading[]): number {
  return [...readings].filter((reading) => reading.level > 0).length;
}

