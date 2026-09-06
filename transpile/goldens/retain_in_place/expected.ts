// MIRRORS: ankurah/retain_in_place/src/input.rs
import { Struct, OwnedClosure, invokeRef, Invocable, dropOwned, HashMap } from '@ankurah/base';

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
    ((<T,>($xs: T[], $p: Invocable<[T], boolean>) => {
      let $at = 0;
      let $i = 0;
      try {
        for (; $i < $xs.length; $i++) {
          if (invokeRef($p, $xs[$i])) { $xs[$at++] = $xs[$i]; } else { dropOwned($xs[$i]); }
        }
      } finally {
        for (; $i < $xs.length; $i++) $xs[$at++] = $xs[$i];
        $xs.length = $at;
        dropOwned($p);
      }
    })(this.items, (item) => item.n >= least));
  }

  keepSet(): void {
    ((<K, V>($m: { [Symbol.iterator](): IterableIterator<[K, V]>; delete(key: K): unknown }, $p: Invocable<[K, V], boolean>) => {
      try {
        for (const [$k, $v] of $m) { if (!invokeRef($p, $k, $v)) $m.delete($k); }
      } finally {
        dropOwned($p);
      }
    })(this.flags, (_, on) => on));
  }

  keepOverGate(gate: Gate): void {
    ((<T,>($xs: T[], $p: Invocable<[T], boolean>) => {
      let $at = 0;
      let $i = 0;
      try {
        for (; $i < $xs.length; $i++) {
          if (invokeRef($p, $xs[$i])) { $xs[$at++] = $xs[$i]; } else { dropOwned($xs[$i]); }
        }
      } finally {
        for (; $i < $xs.length; $i++) $xs[$at++] = $xs[$i];
        $xs.length = $at;
        dropOwned($p);
      }
    })(this.items, new OwnedClosure([gate], (item: Item) => item.n >= gate.least)));
  }

  keepUntilZero(): void {
    ((<T,>($xs: T[], $p: Invocable<[T], boolean>) => {
      let $at = 0;
      let $i = 0;
      try {
        for (; $i < $xs.length; $i++) {
          if (invokeRef($p, $xs[$i])) { $xs[$at++] = $xs[$i]; } else { dropOwned($xs[$i]); }
        }
      } finally {
        for (; $i < $xs.length; $i++) $xs[$at++] = $xs[$i];
        $xs.length = $at;
        dropOwned($p);
      }
    })(this.items, (item) => {
      if (item.n === 0) {
        throw new Error('zero');
      }
      return item.n > 1;
    }));
  }
}

export class Gate extends Struct {
  readonly least: number;

  constructor(least: number) {
    super();
    this.least = least;
  }
}

export function keepOverByValue(items: Item[], least: number): number {
  try {
    ((<T,>($xs: T[], $p: Invocable<[T], boolean>) => {
      let $at = 0;
      let $i = 0;
      try {
        for (; $i < $xs.length; $i++) {
          if (invokeRef($p, $xs[$i])) { $xs[$at++] = $xs[$i]; } else { dropOwned($xs[$i]); }
        }
      } finally {
        for (; $i < $xs.length; $i++) $xs[$at++] = $xs[$i];
        $xs.length = $at;
        dropOwned($p);
      }
    })(items, (item) => {
      if (item.n === 0) {
        throw new Error('zero');
      }
      return item.n >= least;
    }));
    return items.length;
  } finally {
    dropOwned(items);
  }
}

