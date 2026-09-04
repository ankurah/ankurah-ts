// MIRRORS: ankurah/impl_drop/src/input.rs
import { Struct, Drop } from '@ankurah/base';

export class Subscription extends Drop {
  readonly label: string;
  readonly live: boolean;

  constructor(label: string, live: boolean) {
    super();
    this.label = label;
    this.live = live;
  }

  protected override onDrop(): void {
    this.live = false;
  }
}

