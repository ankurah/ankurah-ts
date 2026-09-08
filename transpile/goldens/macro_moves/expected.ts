// MIRRORS: ankurah/macro_moves/src/input.rs
import { Struct } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export class Batch extends Struct {
  readonly entities: Entity[];

  constructor(entities: Entity[]) {
    super();
    this.entities = entities;
  }
}

export function borrow(entity: Entity): number {
  return entity.name.length;
}

export function gather(): Batch {
  let _moved0 = false;
  const first = new Entity('a');
  try {
    const second = new Entity('bb');
    _moved0 = true;
    return new Batch([first, second]);
  } finally {
    if (!_moved0) first.drop();
  }
}

export function describe(): string {
  const first = new Entity('a');
  try {
    const second = new Entity('bb');
    try {
      return `${first.name}:${borrow(second)}`;
    } finally {
      second.drop();
    }
  } finally {
    first.drop();
  }
}

