// MIRRORS: ankurah/loop_ownership/src/input.rs
import { Struct, dropOwned, checkedAdd } from '@ankurah/base';

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

export function drain(entities: Entity[], stopAt: number): number {
  let total = 0;
  const _seq0 = entities;
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const entity = _seq0[_at1++];
      try {
        total = checkedAdd(total, borrow(entity), 'usize');
        if (total > stopAt) {
          break;
        }
      } finally {
        entity.drop();
      }
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  return total;
}

export function consumeAll(entities: Entity[]): number {
  let total = 0;
  const _seq0 = entities;
  let _at1 = 0;
  try {
    while (_at1 < _seq0.length) {
      const entity = _seq0[_at1++];
      total = checkedAdd(total, consume(entity), 'usize');
    }
  } finally {
    dropOwned(_seq0.slice(_at1));
  }
  return total;
}

export function takeUntil(entities: Entity[], stopAt: number): number {
  let total = 0;
  const _seq1 = entities;
  let _at2 = 0;
  try {
    while (_at2 < _seq1.length) {
      const entity = _seq1[_at2++];
      let _moved0 = false;
      try {
        if (entity.name.length > stopAt) {
          break;
        }
        _moved0 = true;
        total = checkedAdd(total, consume(entity), 'usize');
      } finally {
        if (!_moved0) entity.drop();
      }
    }
  } finally {
    dropOwned(_seq1.slice(_at2));
  }
  return total;
}

export function measure(entities: Entity[]): number {
  let total = 0;
  for (const entity of entities) {
    total = checkedAdd(total, borrow(entity), 'usize');
  }
  return total;
}

