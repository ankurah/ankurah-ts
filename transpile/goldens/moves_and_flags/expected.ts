// MIRRORS: ankurah/moves_and_flags/src/input.rs
import { Struct } from '@ankurah/base';

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

