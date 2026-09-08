// MIRRORS: ankurah/a_collection_typed_below/src/input.rs
import { Struct, dropOwned } from '@ankurah/base';

export class Entity extends Struct {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }

  static new(id: bigint): Entity {
    return new Entity(id);
  }
}

export function collectEntities(ids: bigint[]): Entity[] {
  let _moved0 = false;
  let entities = [];
  try {
    for (const id of ids) {
      entities.push(Entity.new(id));
    }
    _moved0 = true;
    return entities;
  } finally {
    if (!_moved0) dropOwned(entities);
  }
}

export function countEntities(ids: bigint[]): number {
  let entities = [];
  try {
    for (const id of ids) {
      entities.push(Entity.new(id));
    }
    return entities.length;
  } finally {
    dropOwned(entities);
  }
}

export function firstId(ids: bigint[]): bigint {
  const entities = collectEntities(ids);
  try {
    return entities[0].id;
  } finally {
    dropOwned(entities);
  }
}

