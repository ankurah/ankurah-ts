// MIRRORS: ankurah/range_shapes/src/input.rs
import { unsupported, iterFirst, range, stepBy } from '@ankurah/base';

export function withinUnit(x: number): boolean {
  return (0.0 <= x && x < 1.0);
}

export function within16(x: number): boolean {
  return (0 <= x && x < 16);
}

export function upTo16(x: number): boolean {
  return (0 <= x && x <= 16);
}

export function evensToTen(): number[] {
  return stepBy((range(0, 10)), 2);
}

export function letters(): string[] {
  return (unsupported('a `char` range is the sequence of its code points, and the port writes a `char` as a one-character string, which `n++` does not step'));
}

export function firstSlot(slots: (number | null)[]): number | null | null {
  return unsupported('`first` answers an `Option` of the element, and this element is itself an `Option`; the port writes both as `null`, so the answer cannot say whether there is no element or an element that is `None`');
}

export function firstPlain(ns: number[]): number | null {
  return iterFirst(ns);
}

