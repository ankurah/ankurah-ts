// MIRRORS: ankurah/signals/src/signal/read.rs
import { Struct, Arc, OwnedClosure } from '@ankurah/base';
import { Broadcast, BroadcastId, ListenerGuard } from '../broadcast';
import { CurrentObserver } from '../context';
import { IntoSubscribeListener_dispatch_intoSubscribeListener, Subscribe, SubscriptionGuard } from '../porcelain/subscribe';
import { Get, GetReadCell, Peek, Signal, With } from '../signal';
import { Memo } from './memo';
import { ReadValueCell, ValueCell } from '../value';

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

  map<Output, Transform>(transform: Transform): Map<Read<T>, T, Output, Transform> {
    return Map.new(this.clone(), transform);
  }

  memo<Output, Transform>(transform: Transform): Memo<Read<T>, T, Output, Transform> {
    return Memo.new(this.clone(), transform);
  }

  clone(): Read<T> {
    return new Read(this.value.clone(), this.broadcast.clone());
  }

  get(): T {
    CurrentObserver.track(this);
    return this.value.value();
  }

  peek(): T {
    return this.value.value();
  }

  with<R>(f: (arg0: T) => R): R {
    CurrentObserver.track(this);
    return this.value.with(f);
  }

  getReadcell(): ReadValueCell<T> {
    return this.value.readvalue();
  }

  listen(listener: Listener): ListenerGuard {
    const _t0 = this.broadcast.reference();
    try {
      return ListenerGuard.new(_t0.listen(new NotifyOnly(Arc.new(new OwnedClosure([listener], () => listener([]))))));
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
    return this.with((selfVal) => other.with((otherVal) => selfVal === otherVal));
  }

  toString(): string {
    return this.with((v) => `${v}`);
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const roValue = this.getReadcell();
    const sigLguard = this.listen(Arc.new(new OwnedClosure([roValue, listener_1], (_) => {
      const currentValue = roValue.value();
      listener_1(currentValue);
    })));
    return SubscriptionGuard.new(sigLguard);
  }
}

