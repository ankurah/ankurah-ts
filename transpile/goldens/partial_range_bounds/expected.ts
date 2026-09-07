// MIRRORS: ankurah/partial_range_bounds/src/input.rs
import { Struct, Drop, filterOwned, rangeContains } from '@ankurah/base';

export class Version extends Drop {
  readonly _0: number | null;

  constructor(_0: number | null) {
    super();
    this._0 = _0;
  }

  equals(other: Version): boolean {
    const _v = [this._0, other._0];
    if ((_v[0] != null) && (_v[1] != null)) {
      const a = _v[0];
      const b = _v[1];
      return a === b;
    } else {
      return false;
    }
  }

  partialCompareTo(other: Version): number | null {
    const _v = [this._0, other._0];
    if ((_v[0] != null) && (_v[1] != null)) {
      const a = _v[0];
      const b = _v[1];
      return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a, b);
    } else {
      return null;
    }
  }

  protected override onDrop(): void {

  }
}

export class Token extends Drop {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function version(n: number): Version {
  return new Version(n);
}

export function unknown(): Version {
  return new Version(null);
}

export function between(v: Version): boolean {
  const _t0 = version(1);
  try {
    const _t1 = version(9);
    try {
      return rangeContains(_t0, _t1, false, v);
    } finally {
      _t1.drop();
    }
  } finally {
    _t0.drop();
  }
}

export function betweenNames(lo: Version, hi: Version, v: Version): boolean {
  return rangeContains(lo, hi, false, v);
}

export function kept(xs: Token[]): Token[] {
  const it = [...xs];
  return filterOwned(it, (t) => t.n > 1);
}

