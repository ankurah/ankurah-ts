// MIRRORS: ankurah/option_default/src/input.rs
import { Struct } from '@ankurah/base';

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

export function make(raw: string): Entity | null {
  if (raw.length === 0) {
    return null;
  }
  return new Entity(raw.toString());
}

export function orFallback(raw: string): Entity {
  const _d0 = new Entity('fallback'.toString());
  const _u1 = make(raw) ?? _d0;
  if (_u1 !== _d0) _d0.drop();
  return _u1;
}

export function orElse(raw: string): Entity {
  return make(raw) ?? (() => new Entity('lazy'.toString()))();
}

export function width(raw: string): number | null {
  const _r0 = make(raw);
  if (_r0 == null) return null;
  const entity = _r0;
  try {
    return borrow(entity);
  } finally {
    entity.drop();
  }
}

export function check(raw: string): number | null {
  const _r0 = make(raw);
  if (_r0 == null) return null;
  _r0.drop();
  return 0;
}

