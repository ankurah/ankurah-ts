// MIRRORS: ankurah/a_returned_write_appends/src/input.rs
import { Struct } from '@ankurah/base';

export class Size extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  toString(): string {
    let _result = '';
    _result += 'Size(';
    if (this.n > 100) {
      _result += 'big)';
      return _result;
    }
    _result += `${this.n})`;
    return _result;
  }
}

export function large(): string {
  const _t0 = new Size(200);
  try {
    return _t0.toString();
  } finally {
    _t0.drop();
  }
}

export function small(): string {
  const _t0 = new Size(7);
  try {
    return _t0.toString();
  } finally {
    _t0.drop();
  }
}

