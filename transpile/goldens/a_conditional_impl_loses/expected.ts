// MIRRORS: ankurah/a_conditional_impl_loses/src/input.rs
import { Struct } from '@ankurah/base';

export class B extends Struct {
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

export function throughTheDeref(): number {
  let w = Wrap.new();
  try {
    w.set(new B());
    return w.deref().go();
  } finally {
    w.drop();
  }
}

