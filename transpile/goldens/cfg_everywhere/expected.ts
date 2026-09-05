// MIRRORS: ankurah/cfg_everywhere/src/input.rs
import { Struct, Enum, checkedAdd } from '@ankurah/base';

export class Bucket extends Struct {
  readonly prefixLen: number;
  readonly guardDisabled: boolean;

  constructor(prefixLen: number, guardDisabled: boolean) {
    super();
    this.prefixLen = prefixLen;
    this.guardDisabled = guardDisabled;
  }

  static new(prefixLen: number): Bucket {
    return new Bucket(prefixLen, false);
  }

  effective(openEnded: boolean): number {
    const effective = openEnded && this.prefixLen > 0 && !this.guardDisabled ? this.prefixLen : 0;
    return effective;
  }

  checked(): number {
    return checkedAdd(this.prefixLen, 1, 'u32');
  }

  describe(mode: Mode): number {
    try {
      return mode.match({
        Fast: () => 0,
        Checked: () => 1,
      });
    } finally {
      mode.drop();
    }
  }
}

export type ModeV = {
  Fast: {};
  Checked: {};
};

export class Mode extends Enum<ModeV> {
}

