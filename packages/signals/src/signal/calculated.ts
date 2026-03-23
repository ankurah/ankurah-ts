// MIRRORS: ankurah/signals/src/signal/calculated.rs
import { Struct, Arc, RwLock, Ref } from '@ankurah/base';
import { Broadcast, BroadcastId, ListenerGuard } from '../broadcast';
import { CurrentObserver } from '../context';
import { Observer } from '../observer';
import { Subscribe, SubscriptionGuard } from '../porcelain/subscribe';
import { Get, GetReadCell, Peek, Signal, With } from '../signal';
import { ReadValueCell, ValueCell } from '../value';

class SubscriptionEntry extends Struct {
  Guard: ListenerGuard;
  markedForRemoval: boolean;

  constructor(Guard: ListenerGuard, markedForRemoval: boolean) {
    super();
    this.Guard = Guard;
    this.markedForRemoval = markedForRemoval;
  }
}

class Inner<T> extends Struct {
  compute: () => T;
  value: ValueCell<T | null>;
  broadcast: Broadcast<void>;
  entries: RwLock<Map<BroadcastId, SubscriptionEntry>>;

  constructor(compute: () => T, value: ValueCell<T | null>, broadcast: Broadcast<void>, entries: RwLock<Map<BroadcastId, SubscriptionEntry>>) {
    super();
    this.compute = compute;
    this.value = value;
    this.broadcast = broadcast;
    this.entries = entries;
  }
}

export class Calculated<T> extends Struct implements Get, Peek, With, GetReadCell, Signal, Subscribe {
  _0: Arc<Inner<T>>;

  constructor(_0: Arc<Inner<T>>) {
    super();
    this._0 = _0;
  }

  static new<T, F>(compute: F): Calculated<T> {
    const inner = Arc.new(new Inner(compute, ValueCell.new(null), Broadcast.new(), new RwLock(new Map())));
    trigger(inner);
    return new Calculated(inner);
  }

  clone(): Calculated<T> {
    return new Calculated(Arc.clone(this._0));
  }

  get(): T {
    CurrentObserver.track(this);
    return this._0.value.value.with((opt) => opt.asRef().clone());
  }

  peek(): T {
    return this._0.value.value.with((opt) => opt.asRef().clone());
  }

  with<R>(f: (arg0: T) => R): R {
    CurrentObserver.track(this);
    return this._0.value.value.with((opt) => f(opt.asRef()));
  }

  getReadcell(): ReadValueCell<T | null> {
    return this._0.value.value.readvalue();
  }

  listen(listener: Listener): ListenerGuard {
    return ListenerGuard.new(this._0.value.broadcast.reference().listen(listener));
  }

  broadcastId(): BroadcastId {
    return this._0.value.broadcast.id();
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const roValue = this._0.value.value.readvalue();
    const subscription = this.listen(Arc.new((_) => {
      const current = roValue.with((opt) => opt.asRef().clone());
      listener(current);
      current.drop();
    }));
    const _ret = SubscriptionGuard.new(subscription);
    roValue.drop();
    listener.drop();
    return _ret;
  }
}

function trigger(inner: Arc<Inner<T>>): void {
  ((entries) => {
    for (const entry of entries.valuesMut()) {
      entry.markedForRemoval = true;
    }
    entries.drop();

  })(inner.entries.write())
  CurrentObserver.set(Arc.clone(inner));
  const newValue = (inner.compute)();
  inner.value.set(newValue);
  CurrentObserver.remove(inner);
  ((entries) => {
    entries.retain((_, entry) => !entry.markedForRemoval);
    entries.drop();
  })(inner.entries.write())
  inner.broadcast.send([]);
  newValue.drop();
}

