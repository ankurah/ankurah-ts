// MIRRORS: ankurah/impl_drop/src/input.rs
import { Struct, Drop } from '@ankurah/base';

export class Subscription extends Drop {
  readonly name: string;
  live: boolean;

  constructor(name: string, live: boolean) {
    super();
    this.name = name;
    this.live = live;
  }

  protected override onDrop(): void {
    this.live = false;
  }
}

