// MIRRORS: ankurah/core/src/property/value/pn_counter.rs
import { Struct, Result, Arc, Weak, debugString } from '@ankurah/base';
import { Entity } from '../../entity';
import { PNBackend } from '../backend/pn_counter';
import { FromActiveType, FromEntity, InitializeWith, PropertyError } from '../traits';

export class PNCounter<I extends Into<PNValue> & From<PNValue> & Copy & Clone> extends Struct implements FromEntity, InitializeWith<I> {
  readonly propertyName: PropertyName;
  readonly backend: Weak<PNBackend>;

  constructor(propertyName: PropertyName, backend: Weak<PNBackend>) {
    super();
    this.propertyName = propertyName;
    this.backend = backend;
  }

  static new<I>(propertyName: PropertyName, backend: Arc<PNBackend>): PNCounter<I> {
    try {
      return new PNCounter(propertyName, backend.downgrade(), undefined /* PhantomData */);
    } finally {
      backend.drop();
    }
  }

  backend(): Arc<PNBackend> {
    return (this.backend.upgrade() ?? (() => { throw new Error('Expected `PN` property backend to exist'); })());
  }

  add(amount: PNValue): void {
    const _t0 = this.backend();
    try {
      _t0.value.add(this.propertyName, amount.asI64());
    } finally {
      _t0.drop();
    }
  }

  value(): I {
    const _t0 = this.backend();
    try {
      const pnValue = _t0.value.get(this.propertyName);
      return I.from(pnValue);
    } finally {
      _t0.drop();
    }
  }

  static fromEntity<I>(propertyName: PropertyName, entity: Entity): PNCounter<I> {
    const backend = entity.getBackend();
    return PNCounter.new(propertyName, backend);
  }

  static initializeWith<I>(entity: Entity, propertyName: PropertyName, value: I): PNCounter<I> {
    const new_ = PNCounter.fromEntity(propertyName, entity);
    try {
      new_.add(value);
      return new_;
    } finally {
      new_.drop();
    }
  }

  debug(): string {
    return `PNCounter { propertyName: ${debugString(this.propertyName)}, backend: ${this.backend}, phantom: ${this.phantom} }`;
  }
}

export function fromActive<I extends PNValue>(active: PNCounter<I>): Result<I, PropertyError> {
  try {
    return Result.Ok(active.value());
  } finally {
    active.drop();
  }
}

