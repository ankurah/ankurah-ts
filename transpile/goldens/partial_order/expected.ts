// MIRRORS: ankurah/partial_order/src/input.rs
import { Struct } from '@ankurah/base';

export class Weight extends Struct {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  equals(other: Weight): boolean {
    return this._0 === other._0;
  }

  compareTo(other: Weight): number {
    return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(this._0, other._0);
  }

  partialCompareTo(other: Weight): number | null {
    if (this._0 === 0 || other._0 === 0) {
      return null;
    }
    return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(this._0, other._0);
  }
}

export class Plain extends Struct {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  equals(other: Plain): boolean {
    return this._0 === other._0;
  }

  compareTo(other: Plain): number {
    return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(this._0, other._0);
  }
}

