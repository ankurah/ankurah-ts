// MIRRORS: ankurah/option_adaptors/src/input.rs
import { Struct, checkedAdd, checkedRem, iterPosition, iterRposition, iterFind, iterFindMap, iterLast, iterFirst, iterMaxByKey, iterMinByKey, iterReduce, iterFilterMap, range } from '@ankurah/base';

export class Watchers extends Struct {
  readonly ids: number[];

  constructor(ids: number[]) {
    super();
    this.ids = ids;
  }

  static new(ids: number[]): Watchers {
    return new Watchers(ids);
  }

  remove(id: number): void {
    {
      const _v = iterPosition([...this.ids], (w) => w === id);
      if (_v != null) {
        const pos = _v;
        this.ids.splice(pos, 1)[0];
      }
    }
  }

  lastAtLeast(atLeast: number): number | null {
    return iterRposition([...this.ids], (w) => w >= atLeast);
  }
}

export class Reading extends Struct {
  readonly label: string;

  constructor(label: string) {
    super();
    this.label = label;
  }

  static new(label: string): Reading {
    return new Reading(label);
  }
}

export function firstOver(ns: number[], over: number): number | null {
  return iterFind([...ns], (n) => n > over);
}

export function firstLabelOver(labels: string[], over: number): string | null {
  return iterFindMap([...labels], (l) => (l.length > over ? l : null));
}

export function ends(ns: number[]): [number | null, number | null] {
  return [iterFirst(ns), iterLast(ns)];
}

export function total(ns: number[]): number | null {
  return iterReduce([...ns], (a, b) => checkedAdd(a, b, 'u32'));
}

export function widest(labels: string[]): string | null {
  return iterMaxByKey([...labels], (l) => l.length);
}

export function narrowest(labels: string[]): string | null {
  return iterMinByKey([...labels], (l) => l.length);
}

export function firstDroppable(readings: Reading[], prefix: string): boolean {
  return (iterFind([...readings], (r) => r.label.startsWith(prefix)) != null);
}

export function counted(to: number): number[] {
  let out = [];
  for (const n of range(0, to)) {
    out.push(n);
  }
  return out;
}

export function evensBackwards(to: number): number[] {
  return iterFilterMap((range(0, to)).slice().reverse(), (n) => (checkedRem(n, 2, 'usize') === 0 ? n : null));
}

