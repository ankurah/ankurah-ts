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
  return new Entity(raw);
}

export function orFallback(raw: string): Entity {
  const _o0 = make(raw);
  const _d1 = new Entity('fallback');
  const _u2 = _o0 ?? _d1;
  if (_u2 !== _d1) _d1.drop();
  return _u2;
}

export function orElse(raw: string): Entity {
  return make(raw) ?? (() => new Entity('lazy'))();
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

