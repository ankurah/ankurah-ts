// MIRRORS: ankurah/partial_move/src/input.rs
import { Struct, checkedAdd } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }

  intoName(): string {
    try {
      return this.name;
    } finally {
      this.drop();
    }
  }

  width(): number {
    return this.name.length;
  }
}

export class Pair extends Struct {
  readonly one: Entity;
  readonly two: Entity;

  constructor(one: Entity, two: Entity) {
    super();
    this.one = one;
    this.two = two;
  }
}

export class Single extends Struct {
  readonly only: Entity;

  constructor(only: Entity) {
    super();
    this.only = only;
  }
}

export function consume(entity: Entity): number {
  try {
    return entity.name.length;
  } finally {
    entity.drop();
  }
}

export function borrow(entity: Entity): number {
  return entity.name.length;
}

export function takeOne(pair: Pair): number {
  try {
    const one = pair.takeField('one');
    const seen = borrow(pair.two);
    return checkedAdd(consume(one), seen, 'usize');
  } finally {
    pair.drop();
  }
}

export function split(pair: Pair): Single {
  try {
    return new Single(pair.takeField('one'));
  } finally {
    pair.drop();
  }
}

export function takeBoth(pair: Pair): number {
  try {
    const one = pair.takeField('one');
    const two = pair.takeField('two');
    return checkedAdd(consume(one), consume(two), 'usize');
  } finally {
    pair.drop();
  }
}

export function nameOfOne(pair: Pair): string {
  try {
    return pair.takeField('one').intoName();
  } finally {
    pair.drop();
  }
}

export function widthOfOne(pair: Pair): number {
  return pair.one.width();
}

