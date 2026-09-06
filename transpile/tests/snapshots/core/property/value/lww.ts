// MIRRORS: ankurah/core/src/property/value/lww.rs
import { Struct, Result, Arc, OwnedClosure, dropOwned } from '@ankurah/base';
import { Listener, ListenerGuard, Signal, BroadcastId, Subscribe, SubscriptionGuard } from '@ankurah/signals';
import { Entity } from '../../entity';
import { Value } from '../../value/index';
import { LWWBackend } from '../backend/lww';
import { Property_dispatch_intoValue } from '../index';
import { FromActiveType, FromEntity, InitializeWith, PropertyError } from '../traits';

export class LWW<T extends Property & Clone> extends Struct implements FromEntity, InitializeWith<T>, Signal, Subscribe<T> {
  readonly propertyName: PropertyName;
  readonly backend: Arc<LWWBackend>;
  readonly entity: Entity;

  constructor(propertyName: PropertyName, backend: Arc<LWWBackend>, entity: Entity) {
    super();
    this.propertyName = propertyName;
    this.backend = backend;
    this.entity = entity;
  }

  set(value: T): Result<void, PropertyError> {
    if (!this.entity.isWritable()) {
      return Result.Err(new PropertyError('TransactionClosed', {}));
    }
    const _r0 = Property_dispatch_intoValue(value);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const value_1 = _r0.unwrap();
    try {
      _moved1 = true;
      this.backend.value.set(this.propertyName, value_1);
      return Result.Ok([]);
    } finally {
      if (!_moved1) dropOwned(value_1);
    }
  }

  get(): Result<T, PropertyError> {
    const value = this.getValue();
    return T.fromValue(value);
  }

  getValue(): Value | null {
    return this.backend.value.get(this.propertyName);
  }

  toString(): Result {
    return f.debugStruct('LWW').field('property_name', this.propertyName).finish();
  }

  static fromEntity<T>(propertyName: PropertyName, entity: Entity): LWW<T> {
    const backend = entity.getBackend().expect('LWW Backend should exist');
    return new LWW(propertyName, backend, entity.clone(), undefined /* PhantomData */);
  }

  static initializeWith<T>(entity: Entity, propertyName: PropertyName, value: T): LWW<T> {
    const new_ = LWW.fromEntity(propertyName, entity);
    try {
      new_.set(value).unwrap();
      return new_;
    } finally {
      new_.drop();
    }
  }

  listen(listener: Listener): ListenerGuard {
    return this.backend.value.listenField(this.propertyName, listener);
  }

  broadcastId(): BroadcastId {
    return this.backend.value.fieldBroadcastId(this.propertyName);
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const lww = this.clone();
    const subscription = this.listen(Arc.new(new OwnedClosure([lww, listener_1], (_) => {
      {
        const _v = lww.get();
        if (_v.isOk()) {
          const currentValue = _v.unwrap();
          listener_1(currentValue);
        }
      }
    }, undefined, true)));
    return SubscriptionGuard.new(subscription);
  }

  clone(): LWW<T> {
    return new LWW(this.propertyName, this.backend.clone(), this.entity.clone(), this.phantom.clone());
  }
}

export function fromActive<T extends Property>(active: LWW<T>): Result<T, PropertyError> {
  try {
    return active.get();
  } finally {
    active.drop();
  }
}

