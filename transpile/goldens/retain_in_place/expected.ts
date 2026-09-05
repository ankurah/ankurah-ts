// MIRRORS: ankurah/retain_in_place/src/input.rs
import { Struct, dropOwned, HashMap } from '@ankurah/base';

export class Item extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Bag extends Struct {
  readonly items: Item[];
  readonly flags: HashMap<string, boolean>;

  constructor(items: Item[], flags: HashMap<string, boolean>) {
    super();
    this.items = items;
    this.flags = flags;
  }

  keepOver(least: number): void {
    (($xs) => { let $at = 0; for (let $i = 0; $i < $xs.length; $i++) { if (((item) => item.n >= least)($xs[$i])) { $xs[$at++] = $xs[$i]; } else { dropOwned($xs[$i]); } } $xs.length = $at; })(this.items);
  }

  keepSet(): void {
    { for (const [_k, _v] of this.flags) { if (!(((_, on) => on)(_k, _v))) this.flags.delete(_k); } };
  }
}

