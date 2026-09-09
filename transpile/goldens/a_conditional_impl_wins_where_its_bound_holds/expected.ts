// MIRRORS: ankurah/a_conditional_impl_wins_where_its_bound_holds/src/input.rs
import { Struct } from '@ankurah/base';

export class R extends Struct implements Red {

  red(): number {
    return 1;
  }
}

export class Inner extends Struct {

  go(): number {
    return 20;
  }
}

export class Wrap<T extends Red> extends Struct implements Ext {
  readonly held: T[];
  readonly inner: Inner;

  constructor(held: T[], inner: Inner) {
    super();
    this.held = held;
    this.inner = inner;
  }

  static new<T>(): Wrap<T> {
    return new Wrap([], new Inner());
  }

  set(value: T): void {
    this.held.push(value);
  }

  deref(): Inner {
    return this.inner;
  }

  go(): number {
    return 99;
  }
}

export interface Red {
  red(): number;
}

export interface Ext {
  go(): number;
}

export function throughTheBound(): number {
  let w = Wrap.new();
  try {
    w.set(new R());
    return w.go();
  } finally {
    w.drop();
  }
}

