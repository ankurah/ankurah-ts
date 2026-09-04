// MIRRORS: ankurah/block_drops/src/input.rs
import { Struct } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export class Registry extends Struct {
  readonly count: number;

  constructor(count: number) {
    super();
    this.count = count;
  }

  static new(): Registry {
    return new Registry(0);
  }

  describe(empty: boolean): number {
    const first = new Entity('');
    try {
      const second = new Entity('');
      try {
        if (empty) {
          return 0;
        }
        return first.name.length + second.name.length;
      } finally {
        second.drop();
      }
    } finally {
      first.drop();
    }
  }

  tally(): number {
    const n = 3;
    return n + this.count;
  }
}

