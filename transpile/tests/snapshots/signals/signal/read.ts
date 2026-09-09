// MIRRORS: ankurah/signals/src/signal/read.rs
import { Struct, Arc, OwnedClosure, invokeRef, Invocable, dropOwned, valueEquals } from '@ankurah/base';
import { Broadcast, BroadcastId, BroadcastListener } from '../broadcast';
import { CurrentObserver } from '../context';
import { IntoSubscribeListener_dispatch_intoSubscribeListener, Subscribe, SubscriptionGuard } from '../porcelain/subscribe';
import { Get, GetReadCell, ListenerGuard, Peek, Signal, With } from '../signal';
import { ReadValueCell, ValueCell } from '../value';
import { Memo } from './memo';

export class Read<T extends Clone & PartialEq & Eq & Display> extends Struct implements Get<T>, Peek<T>, With<T>, GetReadCell<T>, Signal, Subscribe<T> {
  value: ValueCell<T>;
  broadcast: Broadcast<void>;

  constructor(value: ValueCell<T>, broadcast: Broadcast<void>) {
    super();
    this.value = value;
    this.broadcast = broadcast;
  }

  value(): T {
    return this.value.value();
  }

  map<Output, Transform extends Invocable<[T], Output>>(transform: Transform): Map<Read<T>, T, Output, Transform> {
    let _moved0 = false;
    try {
      const _b1 = this.clone();
      _moved0 = true;
      return Map.new(_b1, transform);
    } finally {
      if (!_moved0) dropOwned(transform);
    }
  }

  memo<Output, Transform extends Invocable<[T], Output>>(transform: Transform): Memo<Read<T>, T, Output, Transform> {
    let _moved0 = false;
    try {
      const _b1 = this.clone();
      _moved0 = true;
      return Memo.new(_b1, transform);
    } finally {
      if (!_moved0) dropOwned(transform);
    }
  }

  clone(): Read<T> {
    let _moved1 = false;
    const _b0 = this.value.clone();
    try {
      const _b2 = this.broadcast.clone();
      _moved1 = true;
      return new Read(_b0, _b2);
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  }

  get(): T {
    CurrentObserver.track(this);
    return this.value.value();
  }

  peek(): T {
    return this.value.value();
  }

  with<R>(f: (arg0: T) => R): R {
    let _moved0 = false;
    try {
      CurrentObserver.track(this);
      _moved0 = true;
      return this.value.with(f);
    } finally {
      if (!_moved0) dropOwned(f);
    }
  }

  getReadcell(): ReadValueCell<T> {
    return this.value.readvalue();
  }

  listen(listener: Listener): ListenerGuard {
    const _t0 = this.broadcast.reference();
    try {
      return ListenerGuard.new(_t0.listen(new BroadcastListener('NotifyOnly', { _0: Arc.new(new OwnedClosure([listener], () => invokeRef(listener.value, []))) })));
    } finally {
      _t0.drop();
    }
  }

  broadcastId(): BroadcastId {
    return this.broadcast.id();
  }

  equals(other: Read<T>): boolean {
    if (ptr.eq(this, other)) {
      return true;
    }
    return this.with((selfVal) => other.with((otherVal) => valueEquals(selfVal, otherVal)));
  }

  toString(): string {
    return this.with((v) => `${v}`);
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const roValue = this.getReadcell();
    const sigLguard = this.listen(Arc.new(new OwnedClosure([roValue, listener_1], (_) => {
      const currentValue = roValue.value();
      invokeRef(listener_1, currentValue);
    })));
    return SubscriptionGuard.new(sigLguard);
  }
}

