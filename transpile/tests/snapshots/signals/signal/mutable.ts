// MIRRORS: ankurah/signals/src/signal/mutable.rs
import { Struct, Arc } from '@ankurah/base';
import { Broadcast, BroadcastId, ListenerGuard } from '../broadcast';
import { CurrentObserver } from '../context';
import { Subscribe, SubscriptionGuard } from '../porcelain/subscribe';
import { Get, GetReadCell, Peek, Signal, With } from '../signal';
import { Read } from './read';
import { ReadValueCell, ValueCell } from '../value';

export class Mut<T extends Clone> extends Struct implements Get<T>, Peek<T>, With<T>, GetReadCell<T>, Signal, Subscribe<T> {
  value: ValueCell<T>;
  broadcast: Broadcast<void>;

  constructor(value: ValueCell<T>, broadcast: Broadcast<void>) {
    super();
    this.value = value;
    this.broadcast = broadcast;
  }

  static new<T>(value: T): Mut<T> {
    const broadcast = Broadcast.new();
    return new Mut(ValueCell.new(value), broadcast);
  }

  set(value: T): void {
    this.value.set(value);
    this.broadcast.send([]);
  }

  with<R>(f: (arg0: T) => R): R {
    return this.value.with(f);
  }

  read(): Read<T> {
    return new Read(this.value.clone(), this.broadcast.clone());
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
    return ListenerGuard.new(this.broadcast.reference().listen(listener));
  }

  broadcastId(): BroadcastId {
    return this.broadcast.id();
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = listener.intoSubscribeListener();
    const roValue = this.getReadcell();
    const subscription = this.listen(Arc.new((_) => {
      const currentValue = roValue.value();
      listener_1(currentValue);
      currentValue.drop();
    }));
    const _ret = SubscriptionGuard.new(subscription);
    roValue.drop();
    listener.drop();
    return _ret;
  }

  clone(): Mut<T> {
    return new Mut(this.value.clone(), this.broadcast.clone());
  }
}

