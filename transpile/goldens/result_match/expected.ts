// MIRRORS: ankurah/result_match/src/input.rs
import { Struct, Result } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }
}

export class Failure extends Struct {
  readonly reason: string;

  constructor(reason: string) {
    super();
    this.reason = reason;
  }
}

export function consumeEntity(entity: Entity): number {
  try {
    return entity.name.length;
  } finally {
    entity.drop();
  }
}

export function consumeFailure(failure: Failure): number {
  try {
    return failure.reason.length;
  } finally {
    failure.drop();
  }
}

export function borrowEntity(entity: Entity): number {
  return entity.name.length;
}

export function borrowFailure(failure: Failure): number {
  return failure.reason.length;
}

export function fetch(raw: string): Result<Entity, Failure> {
  if (raw.length === 0) {
    return Result.Err(new Failure('empty'.toString()));
  }
  return Result.Ok(new Entity(raw.toString()));
}

export function width(raw: string): number {
  const _v = fetch(raw);
  if (_v.isOk()) {
    const entity = _v.unwrap();
    return consumeEntity(entity);
  } else {
    const failure = _v.unwrapErr();
    return consumeFailure(failure);
  }
}

export function score(raw: string): number {
  const _v = fetch(raw);
  if (_v.isOk()) {
    const entity = _v.unwrap();
    try {
      return borrowEntity(entity) + 1;
    } finally {
      entity.drop();
    }
  } else {
    const failure = _v.unwrapErr();
    try {
      return borrowFailure(failure) + 100;
    } finally {
      failure.drop();
    }
  }
}

export function orDefault(raw: string): Entity {
  const _v = fetch(raw);
  if (_v.isOk()) {
    const entity = _v.unwrap();
    return entity;
  } else {
    const failure = _v.unwrapErr();
    try {
      return new Entity('fallback'.toString());
    } finally {
      failure.drop();
    }
  }
}

