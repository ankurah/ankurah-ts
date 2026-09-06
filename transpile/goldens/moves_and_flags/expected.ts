// MIRRORS: ankurah/moves_and_flags/src/input.rs
import { Struct, checkedAdd } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export class Pair extends Struct {
  readonly left: Entity;

  constructor(left: Entity) {
    super();
    this.left = left;
  }
}

export class Sink extends Struct {

  swallow(entity: Entity, n: number): number {
    try {
      return checkedAdd(entity.name.length, n, 'usize');
    } finally {
      entity.drop();
    }
  }
}

export class Held extends Struct {
  readonly entity: Entity;
  readonly n: number;

  constructor(entity: Entity, n: number) {
    super();
    this.entity = entity;
    this.n = n;
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

export function movedIntoACall(): number {
  const entity = new Entity('');
  return consume(entity);
}

export function borrowedByACall(): number {
  const entity = new Entity('');
  try {
    return borrow(entity);
  } finally {
    entity.drop();
  }
}

export function movedIntoALiteral(): Pair {
  const entity = new Entity('');
  return new Pair(entity);
}

export function movedOnOnePath(handItOn: boolean): number {
  let _moved0 = false;
  const entity = new Entity('');
  try {
    if (handItOn) {
      _moved0 = true;
      return consume(entity);
    }
    return borrow(entity);
  } finally {
    if (!_moved0) entity.drop();
  }
}

export function droppedByHand(): number {
  const entity = new Entity('');
  entity.drop();
  return 0;
}

export function eat(entity: Entity, n: number): number {
  try {
    return checkedAdd(entity.name.length, n, 'usize');
  } finally {
    entity.drop();
  }
}

export function plainCall(entity: Entity, n: number | null, early: boolean): number {
  let _moved0 = false;
  try {
    if (early) {
      return 0;
    }
    const _b1 = (n ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
    _moved0 = true;
    return eat(entity, _b1);
  } finally {
    if (!_moved0) entity.drop();
  }
}

export function methodCall(sink: Sink, entity: Entity, n: number | null, early: boolean): number {
  let _moved0 = false;
  try {
    if (early) {
      return 0;
    }
    const _b1 = (n ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
    _moved0 = true;
    return sink.swallow(entity, _b1);
  } finally {
    if (!_moved0) entity.drop();
  }
}

export function constructor(entity: Entity, n: number | null, early: boolean): Held {
  let _moved0 = false;
  try {
    if (early) {
      return new Held(new Entity(''), 0);
    }
    const _b1 = (n ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
    _moved0 = true;
    return new Held(entity, _b1);
  } finally {
    if (!_moved0) entity.drop();
  }
}

