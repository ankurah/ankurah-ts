// MIRRORS: ankurah/blanket_free_fn/src/input.rs
import { Struct, Arc, OwnedClosure, invokeRef, dropOwned } from '@ankurah/base';

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
  return IntoListener_dispatch_intoListener(listener);
}

export function intoListener<F extends (arg0: number) => number>(self: F): Listener {
  try {
    return new Listener(invokeRef(self, 1));
  } finally {
    dropOwned(self);
  }
}

export function Arc_Inner_intoListener(self: Arc<Inner>): Listener {
  try {
    return new Listener(self.value.tag);
  } finally {
    self.drop();
  }
}

export function IntoListener_dispatch_intoListener(self: unknown): Listener {
  if (typeof self === 'function' || self instanceof OwnedClosure) return intoListener(self as any);
  if (self instanceof Arc) return Arc_Inner_intoListener(self as any);
  throw new Error(`BUG: no IntoListener impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

