// MIRRORS: ankurah/owned_closures/src/input.rs
import { Struct, OwnedClosure } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export function borrow(entity: Entity): number {
  return entity.name.length;
}

export function runNow(): number {
  const entity = new Entity('abc'.toString());
  return (() => {
    try {
      return borrow(entity);
    } finally {
      entity.drop();
    }
  })();
}

export function runLater(): number {
  const entity = new Entity('abcd'.toString());
  const f = new OwnedClosure([entity], () => borrow(entity));
  try {
    return f.call() + f.call();
  } finally {
    f.drop();
  }
}

export function plain(n: number): number {
  const f = () => n + 1;
  return f();
}

export function borrowing(): number {
  const entity = new Entity('ab'.toString());
  try {
    const f = () => borrow(entity);
    return f();
  } finally {
    entity.drop();
  }
}

