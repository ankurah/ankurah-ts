// MIRRORS: ankurah/partial_move/src/input.rs
import { Struct } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
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
    return consume(one) + seen;
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
    return consume(one) + consume(two);
  } finally {
    pair.drop();
  }
}

