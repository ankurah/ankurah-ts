// MIRRORS: ankurah/consuming_match/src/input.rs
import { Struct, Enum } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export type SlotV = {
  Empty: {};
  Filled: { _0: Entity };
};

export class Slot extends Enum<SlotV> {
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

export function take(slot: Slot): number {
  return slot.intoMatch({
    Empty: () => 0,
    Filled: (v) => {
      const entity = v._0;
      return consume(entity);
    },
  });
}

export function width(slot: Slot): number {
  return slot.intoMatch({
    Empty: () => 0,
    Filled: (v) => {
      const entity = v._0;
      try {
        return borrow(entity) + 1;
      } finally {
        entity.drop();
      }
    },
  });
}

export function intoEntity(slot: Slot): Entity | null {
  return slot.intoMatch({
    Empty: () => null,
    Filled: (v) => {
      const entity = v._0;
      return entity;
    },
  });
}

export function label(slot: Slot): number {
  return slot.intoMatch({
    Empty: () => 0,
    Filled: (v) => {
      const entity = v._0;
      try {
        const width = borrow(entity);
        return width * 2;
      } finally {
        entity.drop();
      }
    },
  });
}

export function peek(slot: Slot): number {
  return slot.match({
    Empty: () => 0,
    Filled: (v) => {
      const entity = v._0;
      return borrow(entity);
    },
  });
}

