// MIRRORS: ankurah/borrowed_iteration/src/input.rs
import { Struct, dropOwned, checkedAdd, HashMap, HashSet, keyHash } from '@ankurah/base';

export class Key extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }

  equals(other: Key): boolean {
    if (this.name !== other.name) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.name)].map((p) => p.length + ':' + p).join('');
  }
}

export class Cell extends Struct {
  readonly value: number;

  constructor(value: number) {
    super();
    this.value = value;
  }
}

export class Ordering extends Struct {
  readonly keys: Key[] | null;

  constructor(keys: Key[] | null) {
    super();
    this.keys = keys;
  }
}

export function sumBorrowed(map: HashMap<Key, Cell>): number {
  let total = 0;
  for (const [_key, cell] of map) {
    total = checkedAdd(total, cell.value, 'u32');
  }
  return total;
}

export function sumAmp(map: HashMap<Key, Cell>): number {
  try {
    let total = 0;
    for (const [_key, cell] of map) {
      total = checkedAdd(total, cell.value, 'u32');
    }
    return total;
  } finally {
    dropOwned(map);
  }
}

export function sumConsuming(map: HashMap<Key, Cell>): number {
  let total = 0;
  for (const [_key, cell] of map) {
    try {
      try {
        total = checkedAdd(total, cell.value, 'u32');
      } finally {
        cell.drop();
      }
    } finally {
      _key.drop();
    }
  }
  return total;
}

export function widths(keys: Key[]): number {
  let total = 0;
  for (const key of keys) {
    total = checkedAdd(total, key.name.length, 'usize');
  }
  return total;
}

export function firstWidth(keys: Key[]): number {
  let total = 0;
  const _seq0 = keys;
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const key = _seq0[_at1++];
      try {
        total = checkedAdd(total, key.name.length, 'usize');
        break;
      } finally {
        key.drop();
      }
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  return total;
}

export function orderingWidth(o: Ordering): number {
  let total = 0;
  {
    const _v = o.keys;
    if (_v != null) {
      const keys = _v;
      for (const key of keys) {
        total = checkedAdd(total, key.name.length, 'usize');
      }
    }
  }
  return total;
}

export function refWidths(keys: Key[]): number {
  let total = 0;
  const _seq0 = keys;
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const key = _seq0[_at1++];
      try {
        total = checkedAdd(total, key.name.length, 'usize');
      } finally {
        key.drop();
      }
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  return total;
}

export function refWidthsBorrowed(keys: Key[]): number {
  let total = 0;
  for (const key of keys) {
    total = checkedAdd(total, key.name.length, 'usize');
  }
  return total;
}

