// MIRRORS: ankurah/a_closure_body_typed_by_its_position/src/input.rs
import { Struct, invokeRef, Invocable, dropOwned } from '@ankurah/base';

export class Tag extends Struct {
  readonly text: string;

  constructor(text: string) {
    super();
    this.text = text;
  }
}

export class Entity extends Struct {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }

  tag(): Tag {
    return new Tag('e');
  }
}

export class Bus extends Struct {
  readonly entities: Entity[];

  constructor(entities: Entity[]) {
    super();
    this.entities = entities;
  }

  each(f: Invocable<[Entity], number>): number {
    try {
      let total = 0;
      for (const entity of this.entities) {
        total += invokeRef(f, entity);
      }
      return total;
    } finally {
      dropOwned(f);
    }
  }
}

export function overEach(entities: Entity[], f: Invocable<[Entity], number>): number {
  try {
    let total = 0;
    for (const entity of entities) {
      total += invokeRef(f, entity);
    }
    return total;
  } finally {
    dropOwned(f);
  }
}

export function tagLengthsThroughAMethod(bus: Bus): number {
  return bus.each((entity) => {
    let tags = [];
    try {
      tags.push(entity.tag());
      return tags.length;
    } finally {
      dropOwned(tags);
    }
  });
}

export function tagLengthsThroughAFunction(entities: Entity[]): number {
  return overEach(entities, (entity) => {
    let tags = [];
    try {
      tags.push(entity.tag());
      return tags.length;
    } finally {
      dropOwned(tags);
    }
  });
}

