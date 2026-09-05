// MIRRORS: ankurah/owned_closures/src/input.rs
import { Struct, OwnedClosure, invoke, invokeRef, Invocable, dropOwned, checkedAdd } from '@ankurah/base';

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
  const entity = new Entity('abc');
  return (() => {
    try {
      return borrow(entity);
    } finally {
      entity.drop();
    }
  })();
}

export function runLater(): number {
  const entity = new Entity('abcd');
  const f = new OwnedClosure([entity], () => borrow(entity));
  try {
    return f.call() + f.call();
  } finally {
    f.drop();
  }
}

export function plain(n: number): number {
  const f = () => checkedAdd(n, 1, 'usize');
  return invokeRef(f);
}

export function borrowing(): number {
  const entity = new Entity('ab');
  try {
    const f = () => borrow(entity);
    return invokeRef(f);
  } finally {
    entity.drop();
  }
}

export function consumed(entity: Entity): number {
  const take = new OwnedClosure([entity], () => {
    const held = entity;
    try {
      return borrow(held);
    } finally {
      held.drop();
    }
  }, undefined, true);
  return take.callOnce();
}

export function throughABound(f: Invocable<[number], number>, n: number): number {
  return invoke(f, n);
}

export function handsAWrappedOne(entity: Entity): number {
  return throughABound(new OwnedClosure([entity], (n) => n + entity.name.length), 1);
}

export function handsAPlainOne(n: number): number {
  return throughABound((x) => x + 1, n);
}

export function twiceByValue(f: Invocable<[number], number>, n: number): number {
  try {
    return invokeRef(f, n) + invokeRef(f, n);
  } finally {
    dropOwned(f);
  }
}

export function twiceByReference(f: Invocable<[number], number>, n: number): number {
  return invokeRef(f, n) + invokeRef(f, n);
}

