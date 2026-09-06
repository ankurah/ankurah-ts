// MIRRORS: ankurah/core/src/property/value/yrs.rs
import { Struct, Result, Arc, OwnedClosure, dropOwned, debugString } from '@ankurah/base';
import { Listener, ListenerGuard, Signal, BroadcastId, Subscribe, SubscriptionGuard } from '@ankurah/signals';
import { Entity } from '../../entity';
import { MutationError } from '../../error';
import { YrsBackend } from '../backend/yrs';
import { FromActiveType, FromEntity, InitializeWith, PropertyError } from '../traits';

export class YrsString<Projected extends Clone> extends Struct implements FromEntity, InitializeWith<string>, InitializeWith<string | null>, Signal, Subscribe<string> {
  readonly propertyName: PropertyName;
  readonly backend: Arc<YrsBackend>;
  readonly entity: Entity;

  constructor(propertyName: PropertyName, backend: Arc<YrsBackend>, entity: Entity) {
    super();
    this.propertyName = propertyName;
    this.backend = backend;
    this.entity = entity;
  }

  static new<Projected>(propertyName: PropertyName, backend: Arc<YrsBackend>, entity: Entity): YrsString<Projected> {
    return new YrsString(propertyName, backend, entity, undefined /* PhantomData */);
  }

  value(): string | null {
    return this.backend.value.getString(this.propertyName);
  }

  insert(index: number, value: string): Result<void, MutationError> {
    if (!this.entity.isWritable()) {
      return Result.Err(MutationError.fromPropertyError(new PropertyError('TransactionClosed', {})));
    }
    return this.backend.value.insert(this.propertyName, index, value);
  }

  delete(index: number, length: number): Result<void, MutationError> {
    if (!this.entity.isWritable()) {
      return Result.Err(MutationError.fromPropertyError(new PropertyError('TransactionClosed', {})));
    }
    return this.backend.value.delete(this.propertyName, index, length);
  }

  overwrite(start: number, length: number, value: string): Result<void, MutationError> {
    if (!this.entity.isWritable()) {
      return Result.Err(MutationError.fromPropertyError(new PropertyError('TransactionClosed', {})));
    }
    const _r0 = this.backend.value.delete(this.propertyName, start, length);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    const _r1 = this.backend.value.insert(this.propertyName, start, value);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    _r1.drop();
    return Result.Ok([]);
  }

  replace(value: string): Result<void, MutationError> {
    if (!this.entity.isWritable()) {
      return Result.Err(MutationError.fromPropertyError(new PropertyError('TransactionClosed', {})));
    }
    const _r0 = this.backend.value.delete(this.propertyName, 0, (this.value() ?? '').length);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    const _r1 = this.backend.value.insert(this.propertyName, 0, value);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    _r1.drop();
    return Result.Ok([]);
  }

  static fromEntity<Projected>(propertyName: PropertyName, entity: Entity): YrsString<Projected> {
    const backend = entity.getBackend().expect('YrsBackend should exist');
    return YrsString.new(propertyName, backend, entity.clone());
  }

  static initializeWith<Projected>(entity: Entity, propertyName: PropertyName, value: string): YrsString<Projected> {
    const newString = YrsString.fromEntity(propertyName, entity);
    newString.insert(0, value).unwrap();
    return newString;
  }

  listen(listener: Listener): ListenerGuard {
    return this.backend.value.listenField(this.propertyName, listener);
  }

  broadcastId(): BroadcastId {
    return this.backend.value.fieldBroadcastId(this.propertyName);
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const yrsString = this.clone();
    const subscription = this.listen(Arc.new(new OwnedClosure([yrsString, listener_1], (_) => {
      {
        const _v = yrsString.value();
        if (_v != null) {
          const currentValue = _v;
          listener_1(currentValue);
        }
      }
    }, undefined, true)));
    return SubscriptionGuard.new(subscription);
  }

  clone(): YrsString<Projected> {
    return new YrsString(this.propertyName, this.backend.clone(), this.entity.clone(), this.phantom.clone());
  }

  debug(): string {
    return `YrsString { propertyName: ${debugString(this.propertyName)}, backend: ${this.backend.value.debug()}, entity: ${this.entity.debug()}, phantom: ${this.phantom} }`;
  }
}

export function Option_fromActive<Projected, S extends FromActiveType>(active: YrsString<Projected>): Result<S | null, PropertyError> {
  const _v = S.fromActive(active);
  if (_v.isOk()) {
    const value = _v.unwrap();
    return Result.Ok(value);
  } else {
    const _v1 = _v.unwrapErr();
    if (_v1.is('Missing')) {
      const _v2 = _v1;
      try {
        return Result.Ok(null);
      } finally {
        dropOwned(_v2);
      }
    }
    {
      const err = _v1;
      return Result.Err(err);
    }
  }
}

export function String_fromActive<Projected>(active: YrsString<Projected>): Result<string, PropertyError> {
  try {
    const _v = active.value();
    if (_v != null) {
      const value = _v;
      return Result.Ok(value);
    } else {
      return Result.Err(new PropertyError('Missing', {}));
    }
  } finally {
    active.drop();
  }
}

export function Cow_Str_fromActive<Projected>(active: YrsString<Projected>): Result<Cow<string>, PropertyError> {
  try {
    const _v = active.value();
    if (_v != null) {
      const value = _v;
      return Result.Ok(Cow.from(value));
    } else {
      return Result.Err(new PropertyError('Missing', {}));
    }
  } finally {
    active.drop();
  }
}

