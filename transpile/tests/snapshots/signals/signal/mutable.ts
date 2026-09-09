// MIRRORS: ankurah/signals/src/signal/mutable.rs
import { Struct, Arc, OwnedClosure, invokeRef, dropOwned } from '@ankurah/base';
import { Broadcast, BroadcastId } from '../broadcast';
import { CurrentObserver } from '../context';
import { IntoSubscribeListener_dispatch_intoSubscribeListener, Subscribe, SubscriptionGuard } from '../porcelain/subscribe';
import { Get, GetReadCell, ListenerGuard, Peek, Signal, With } from '../signal';
import { ReadValueCell, ValueCell } from '../value';
import { Read } from './read';

export class Mut<T extends Clone> extends Struct implements Get<T>, Peek<T>, With<T>, GetReadCell<T>, Signal, Subscribe<T> {
  value: ValueCell<T>;
  broadcast: Broadcast<void>;

  constructor(value: ValueCell<T>, broadcast: Broadcast<void>) {
    super();
    this.value = value;
    this.broadcast = broadcast;
  }

  static new<T>(value: T): Mut<T> {
    let _moved0 = false;
    const broadcast = Broadcast.new();
    try {
      const _b1 = ValueCell.new(value);
      _moved0 = true;
      return new Mut(_b1, broadcast);
    } finally {
      if (!_moved0) broadcast.drop();
    }
  }

  set(value: T): void {
    this.value.set(value);
    this.broadcast.send([]);
  }

  with<R>(f: (arg0: T) => R): R {
    return this.value.with(f);
  }

  read(): Read<T> {
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

  value(): T {
    return this.value.value();
  }

  get(): T {
    CurrentObserver.track(this);
    return this.value.value();
  }

  peek(): T {
    return this.value.value();
  }

  getReadcell(): ReadValueCell<T> {
    return this.value.readvalue();
  }

  listen(listener: Listener): ListenerGuard {
    const _t0 = this.broadcast.reference();
    try {
      return ListenerGuard.new(_t0.listen(listener));
    } finally {
      _t0.drop();
    }
  }

  broadcastId(): BroadcastId {
    return this.broadcast.id();
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const roValue = this.getReadcell();
    const subscription = this.listen(Arc.new(new OwnedClosure([roValue, listener_1], (_) => {
      const currentValue = roValue.value();
      invokeRef(listener_1, currentValue);
    })));
    return SubscriptionGuard.new(subscription);
  }

  clone(): Mut<T> {
    return new Mut(this.value.clone(), this.broadcast.clone());
  }
}

