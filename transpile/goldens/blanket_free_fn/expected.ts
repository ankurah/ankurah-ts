// MIRRORS: ankurah/blanket_free_fn/src/input.rs
import { Struct, Arc } from '@ankurah/base';

export class Listener extends Struct {
  readonly tag: number;

  constructor(tag: number) {
    super();
    this.tag = tag;
  }
}

export class Inner extends Struct {
  readonly tag: number;

  constructor(tag: number) {
    super();
    this.tag = tag;
  }
}

export interface IntoListener {
  intoListener(): Listener;
}

export function fromWrapped(inner: Arc<Inner>): Listener {
  return Arc_Inner_intoListener(inner);
}

export function fromAny<L extends IntoListener>(listener: L): Listener {
  return intoListener(listener);
}

export function intoListener<F extends (arg0: number) => number>(self: F): Listener {
  return new Listener(self(1));
}

export function Arc_Inner_intoListener(self: Arc<Inner>): Listener {
  try {
    return new Listener(self.value.tag);
  } finally {
    self.drop();
  }
}

