// MIRRORS: ankurah/a_loop_pattern_from_an_inferred_element/src/input.rs
import { Struct, dropOwned } from '@ankurah/base';

export class Entity extends Struct {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }
}

export class Event extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export function describe(entityEvents: [Entity, Event][]): string[] {
  let _moved0 = false;
  try {
    let lines = [];
    _moved0 = true;
    const _seq1 = entityEvents;
    let _at2 = 0;
    try {
      while (_at2 < _seq1.length) {
        const [entity, event] = _seq1[_at2++];
        try {
          try {
            lines.push(event.name);
            lines.push(`${entity.id}`);
          } finally {
            event.drop();
          }
        } finally {
          entity.drop();
        }
      }
    } finally {
      dropOwned(_seq1.slice(_at2));
    }
    return lines;
  } finally {
    if (!_moved0) dropOwned(entityEvents);
  }
}

export function describeBuilt(): string[] {
  let _moved0 = false;
  let entityEvents: [Entity, Event][] = [];
  try {
    let _moved2 = false;
    const _b1 = new Entity(1n);
    try {
      const _b3 = new Event('created');
      _moved2 = true;
      entityEvents.push([_b1, _b3]);
    } finally {
      if (!_moved2) dropOwned(_b1);
    }
    let lines = [];
    _moved0 = true;
    const _seq4 = entityEvents;
    let _at5 = 0;
    try {
      while (_at5 < _seq4.length) {
        const [entity, event] = _seq4[_at5++];
        try {
          try {
            lines.push(event.name);
            lines.push(`${entity.id}`);
          } finally {
            event.drop();
          }
        } finally {
          entity.drop();
        }
      }
    } finally {
      dropOwned(_seq4.slice(_at5));
    }
    return lines;
  } finally {
    if (!_moved0) dropOwned(entityEvents);
  }
}

export function collected(): number {
  return describeBuilt().length;
}

