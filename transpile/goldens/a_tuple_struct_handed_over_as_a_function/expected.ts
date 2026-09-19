// MIRRORS: ankurah/a_tuple_struct_handed_over_as_a_function/src/input.rs
import { Struct } from '@ankurah/base';

export class Wrapper extends Struct {
  readonly _0: bigint;

  constructor(_0: bigint) {
    super();
    this._0 = _0;
  }
}

export class Holder extends Struct {
  readonly one: bigint | null;
  readonly many: bigint[];

  constructor(one: bigint | null, many: bigint[]) {
    super();
    this.one = one;
    this.many = many;
  }

  wrapped(): Wrapper | null {
    return (this.one != null ? (((_0) => new Wrapper(_0)))(this.one!) : null);
  }

  all(): Wrapper[] {
    return [...[...this.many]].map(((_0) => new Wrapper(_0)));
  }

  spelled(): Wrapper | null {
    return (this.one != null ? ((n) => new Wrapper(n))(this.one!) : null);
  }
}

