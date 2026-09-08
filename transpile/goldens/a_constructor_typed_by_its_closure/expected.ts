// MIRRORS: ankurah/a_constructor_typed_by_its_closure/src/input.rs
import { Struct, invokeRef, Invocable, dropOwned } from '@ankurah/base';

export class Calculated<T> extends Struct {
  value: T;

  constructor(value: T) {
    super();
    this.value = value;
  }

  static new<T>(compute: Invocable<[], T>): Calculated<T> {
    try {
      return new Calculated(invokeRef(compute));
    } finally {
      dropOwned(compute);
    }
  }

  intoValue(): T {
    try {
      return this.value;
    } finally {
      this.drop();
    }
  }

  isReady(): boolean {
    return true;
  }
}

export function width(text: string): number {
  const counted = Calculated.new(() => text.length);
  return counted.intoValue();
}

export function builtAndKept(text: string): boolean {
  const counted = Calculated.new(() => text.length);
  try {
    return counted.isReady();
  } finally {
    counted.drop();
  }
}

