// MIRRORS: ankurah/repeated_element/src/input.rs
import { Struct, unsupported } from '@ankurah/base';

export class Held extends Struct {
  readonly flag: boolean;

  constructor(flag: boolean) {
    super();
    this.flag = flag;
  }

  static from(flag: boolean): Held {
    return new Held(flag);
  }

  clone(): Held {
    return new Held(this.flag);
  }
}

export function shared(): Held[] {
  return unsupported('this `vec![v; n]` repeats a value the port writes as a reference, and Rust clones it into every slot; `fill` would put ONE object in all of them, and the port has no spelling for evaluate-once-then-clone');
}

export function listed(): Held[] {
  return [Held.from(true), Held.from(false)];
}

export function numbers(): number[] {
  return Array(3).fill(7);
}

export function words(): string[] {
  return Array(2).fill('');
}

