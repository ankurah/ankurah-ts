// MIRRORS: ankurah/literal_match_moves/src/input.rs
import { Struct } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
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

export function byFlag(handItOn: boolean): number {
  let _moved0 = false;
  const entity = new Entity('');
  try {
    if (handItOn === true) {
      _moved0 = true;
      return consume(entity);
    } else {
      return borrow(entity);
    }
  } finally {
    if (!_moved0) entity.drop();
  }
}

export function byNumber(which: number): number {
  let _moved0 = false;
  const entity = new Entity('');
  try {
    if (which === 0) {
      return borrow(entity);
    } else if (which === 1) {
      _moved0 = true;
      return consume(entity);
    } else {
      return borrow(entity) + 1;
    }
  } finally {
    if (!_moved0) entity.drop();
  }
}

