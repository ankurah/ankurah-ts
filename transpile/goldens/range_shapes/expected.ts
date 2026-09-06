// MIRRORS: ankurah/range_shapes/src/input.rs
import { unsupported, iterFind, iterFirst, range, rangeContains, stepBy } from '@ankurah/base';

export function withinUnit(x: number): boolean {
  return rangeContains(0.0, 1.0, false, x);
}

export function within16(x: number): boolean {
  return rangeContains(0, 16, false, x);
}

export function upTo16(x: number): boolean {
  return rangeContains(0, 16, true, x);
}

export function evensToTen(): number[] {
  return stepBy((range(0, 10)), 2);
}

export function letters(): string[] {
  return (unsupported('a `char` range is the sequence of its code points, and the port writes a `char` as a one-character string, which `n++` does not step'));
}

export function side(): number {
  return 3;
}

export function onceOnly(n: number): boolean {
  return rangeContains(0, n, false, side());
}

export function fromFive(x: number): boolean {
  return rangeContains(5, null, false, x);
}

export function upToFive(x: number): boolean {
  return rangeContains(null, 5, false, x);
}

export function upToFiveIncl(x: number): boolean {
  return rangeContains(null, 5, true, x);
}

export function anything(x: number): boolean {
  return rangeContains(null, null, false, x);
}

export function firstSlot(slots: (number | null)[]): number | null | null {
  return unsupported('`first` answers an `Option` of the element, and this element is itself an `Option`; the port writes both as `null`, so the answer cannot say whether there is no element or an element that is `None`');
}

export function firstPlain(ns: number[]): number | null {
  return iterFirst(ns);
}

export function foundSlot(slots: (number | null)[]): number | null | null {
  return unsupported('`find` answers an `Option` of the element, and this element is itself an `Option`; the port writes both as `null`, so the answer cannot say whether there is no element or an element that is `None`');
}

export function takenSlot(slots: (number | null)[]): number | null | null {
  return unsupported('`find` answers an `Option` of the element, and this element is itself an `Option`; the port writes both as `null`, so the answer cannot say whether there is no element or an element that is `None`');
}

export function foundPlain(ns: number[]): number | null {
  return iterFind([...ns], (n) => n > 7);
}

