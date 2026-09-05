// MIRRORS: ankurah/consuming_match/src/input.rs
import { Struct, Enum, dropOwned, checkedAdd, checkedMul } from '@ankurah/base';

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
        return checkedAdd(borrow(entity), 1, 'usize');
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
        return checkedMul(width, 2, 'usize');
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

export function untilFilled(slots: Slot[]): number {
  let seen = 0;
  const _seq2 = slots;
  let _at3 = 0;
  try {
    while (_at3 < _seq2.length) {
      const slot = _seq2[_at3++];
      const _m1 = slot.intoMatch<any>({
        Filled: (v) => {
          const entity = v._0;
          let _moved0 = false;
          try {
            _moved0 = true;
            entity.drop();
            return { $jump: 'break' };
          } finally {
            if (!_moved0) entity.drop();
          }
        },
        Empty: () => {
          seen = checkedAdd(seen, 1, 'i32');
        },
      });
      if ((_m1 as any)?.$jump === 'break') break;
    }
  } finally {
    dropOwned(_seq2.slice(_at3));
  }
  return seen;
}

export function countEmpty(slots: Slot[]): number {
  let seen = 0;
  const _seq2 = slots;
  let _at3 = 0;
  try {
    while (_at3 < _seq2.length) {
      const slot = _seq2[_at3++];
      const _m1 = slot.intoMatch<any>({
        Filled: (v) => {
          const entity = v._0;
          let _moved0 = false;
          try {
            _moved0 = true;
            entity.drop();
            return { $jump: 'continue' };
          } finally {
            if (!_moved0) entity.drop();
          }
        },
        Empty: () => {},
      });
      if ((_m1 as any)?.$jump === 'continue') continue;
      seen = checkedAdd(seen, 1, 'i32');
    }
  } finally {
    dropOwned(_seq2.slice(_at3));
  }
  return seen;
}

