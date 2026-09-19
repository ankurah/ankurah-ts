// MIRRORS: ankurah/a_cloned_value_is_not_an_alias/src/input.rs
import { Struct, HashMap } from '@ankurah/base';

export class Dot extends Struct {
  x: bigint;

  constructor(x: bigint) {
    super();
    this.x = x;
  }

  clone(): Dot {
    return new Dot(this.x);
  }
}

export function writeThroughASequence(map: HashMap<bigint, bigint[]>, k: bigint): bigint {
  const _m0 = map.get(k);
  {
    const _v = (_m0 != null ? [..._m0] : null);
    if (_v != null) {
      const taken = _v;
      taken[0] = 5n;
    }
  }
  const _v1 = map.get(k);
  if (_v1 != null) {
    const v = _v1;
    return v[0];
  } else {
    return 0n;
  }
}

export function writeThroughACopyStruct(map: HashMap<bigint, Dot>, k: bigint): bigint {
  {
    const _v = map.get(k)?.clone() ?? null;
    if (_v != null) {
      const taken = _v;
      taken.x = 5n;
    }
  }
  const _v1 = map.get(k);
  if (_v1 != null) {
    const d = _v1;
    return d.x;
  } else {
    return 0n;
  }
}

export function writeThroughACollection(map: HashMap<bigint, Dot>, k: bigint): bigint {
  let taken = [...map.values()].map((e) => e.clone());
  taken[0].x = 5n;
  const _v = map.get(k);
  if (_v != null) {
    const d = _v;
    return d.x;
  } else {
    return 0n;
  }
}

