// MIRRORS: ankurah/assignment_drops/src/input.rs
import { Struct, Mutex } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export class Holder extends Struct {
  inner: Entity;

  constructor(inner: Entity) {
    super();
    this.inner = inner;
  }
}

export function borrow(entity: Entity): number {
  return entity.name.length;
}

export function replace(first: string, second: string): number {
  let entity = new Entity(first.toString());
  try {
    const _a0 = new Entity(second.toString());
    entity.drop();
    entity = _a0;
    return borrow(entity);
  } finally {
    entity.drop();
  }
}

export function maybeReplace(swap: boolean): number {
  let entity = new Entity('a'.toString());
  try {
    if (swap) {
      const _a0 = new Entity('bb'.toString());
      entity.drop();
      entity = _a0;
    }
    return borrow(entity);
  } finally {
    entity.drop();
  }
}

export function setField(holder: Holder, name: string): number {
  const _a0 = new Entity(name.toString());
  holder.inner.drop();
  holder.inner = _a0;
  return borrow(holder.inner);
}

export function setThroughGuard(cell: Mutex<Entity>, name: string): number {
  let guard = cell.lock();
  try {
    guard.value = new Entity(name.toString());
    return guard.value.name.length;
  } finally {
    guard.drop();
  }
}

