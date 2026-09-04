// MIRRORS: ankurah/param_and_self/src/input.rs
import { Struct } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export class Holder extends Struct {
  readonly inner: Entity;
  readonly spare: Entity;

  constructor(inner: Entity, spare: Entity) {
    super();
    this.inner = inner;
    this.spare = spare;
  }

  width(): number {
    return borrow(this.inner) + borrow(this.spare);
  }

  intoInner(): Entity {
    try {
      return this.takeField('inner');
    } finally {
      this.drop();
    }
  }

  widthOwned(): number {
    try {
      return borrow(this.inner);
    } finally {
      this.drop();
    }
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

export function forward(entity: Entity, handItOn: boolean): number {
  let _moved0 = false;
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

