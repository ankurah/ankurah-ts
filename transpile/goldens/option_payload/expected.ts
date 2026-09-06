// MIRRORS: ankurah/option_payload/src/input.rs
import { Struct, Enum, Drop, unsupported } from '@ankurah/base';

export class Token extends Drop {
  readonly _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  protected override onDrop(): void {

  }
}

export type ValueV = {
  Held: { _0: Token };
  Text: { _0: string };
  Empty: {};
};

export class Value extends Enum<ValueV> {
}

export function sink(t: Token): number {
  try {
    return t._0;
  } finally {
    t.drop();
  }
}

export function read(value: Value | null): number {
  if (value != null) {
    return value.intoMatch({
      Held: (v) => {
        const token = v._0;
        return sink(token);
      },
      Text: (v) => {
        const s = v._0;
        return s.length;
      },
      Empty: (v) => {
        const other = new Value('Empty', v);
        return hold(other);
      },
    });
  } else {
    return 0;
  }

}

export function hold(v: Value): number {
  try {
    return 7;
  } finally {
    v.drop();
  }
}

export function readExact(value: Value | null): number {
  if (value != null) {
    return value.intoMatch({
      Held: (v) => {
        const token = v._0;
        return sink(token);
      },
      Text: (v) => {
        const s = v._0;
        return s.length;
      },
      Empty: () => 1,
    });
  } else {
    return 0;
  }

}

export function peek(value: Value | null): number {
  if (value != null && (value.is('Held'))) {
    return 1;
  } else if (value != null) {
    return 2;
  } else {
    return 0;
  }
}

export function readLoosely(value: Value | null): number {
  unsupported('an arm of this consuming `Option` match tests inside the payload, and the port cannot both take a name out of that payload and release what is left of it here');
}

