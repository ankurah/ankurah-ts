// MIRRORS: ankurah/a_default_the_body_overrules/src/input.rs
import { Struct, checkedAdd, wrappingAdd } from '@ankurah/base';

export class Thing extends Struct {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }

  static new(id: bigint): Thing {
    return new Thing(id);
  }
}

export class Slot<A, B = void> extends Struct {
  readonly a: A;
  readonly b: B;

  constructor(a: A, b: B) {
    super();
    this.a = a;
    this.b = b;
  }

  static new<A, B>(a: A, b: B): Slot<A, B> {
    return new Slot(a, b);
  }
}

export function held(): bigint {
  const s = Slot.new(1, Thing.new(9n));
  try {
    return checkedAdd(s.b.id, BigInt(s.a), 'u64');
  } finally {
    s.drop();
  }
}

export function wrapped(): number {
  const s = Slot.new(1, 200);
  try {
    return wrappingAdd(s.b, 100, 'u8');
  } finally {
    s.drop();
  }
}

export function empty(): number {
  const s = Slot.new(2, []);
  try {
    return s.a;
  } finally {
    s.drop();
  }
}

export function run(): bigint {
  return checkedAdd(checkedAdd(held(), BigInt(wrapped()), 'u64'), BigInt(empty()), 'u64');
}

