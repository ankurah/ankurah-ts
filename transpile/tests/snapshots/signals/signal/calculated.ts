// MIRRORS: ankurah/signals/src/signal/calculated.rs
import { Struct, Arc, RwLock, OwnedClosure } from '@ankurah/base';
import { Broadcast, BroadcastId, ListenerGuard } from '../broadcast';
import { CurrentObserver } from '../context';
import { Observer } from '../observer';
import { IntoSubscribeListener_dispatch_intoSubscribeListener, Subscribe, SubscriptionGuard } from '../porcelain/subscribe';
import { Get, GetReadCell, Peek, Signal, With } from '../signal';
import { ReadValueCell, ValueCell } from '../value';

class SubscriptionEntry extends Struct {
  _guard: ListenerGuard;
  markedForRemoval: boolean;

  constructor(_guard: ListenerGuard, markedForRemoval: boolean) {
    super();
    this._guard = _guard;
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

export class Calculated<T extends Clone> extends Struct implements Get<T>, Peek<T>, With<T>, GetReadCell<T | null>, Signal, Subscribe<T> {
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
    return new Calculated(this._0.clone());
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
    const _t0 = this._0.value.broadcast.reference();
    try {
      return ListenerGuard.new(_t0.listen(listener));
    } finally {
      _t0.drop();
    }
  }

  broadcastId(): BroadcastId {
    return this._0.value.broadcast.id();
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const roValue = this._0.value.value.readvalue();
    const subscription = this.listen(Arc.new(new OwnedClosure([roValue, listener_1], (_) => {
      const current = roValue.with((opt) => opt.asRef().clone());
      listener_1(current);
    })));
    return SubscriptionGuard.new(subscription);
  }
}

function trigger<T>(inner: Arc<Inner<T>>): void {
  (() => {
    let entries = inner.value.entries.write();
    try {
      for (const entry of entries.value.values()) {
        entry.markedForRemoval = true;
      }
    } finally {
      entries.drop();
    }
  })()
  CurrentObserver.set(inner.clone());
  const newValue = (inner.value.compute)();
  inner.value.value.set(newValue);
  CurrentObserver.remove(inner);
  (() => {
    let entries = inner.value.entries.write();
    try {
      { for (const [_k, _v] of entries.value) { if (!((_, entry) => !entry.markedForRemoval(_k, _v))) entries.value.delete(_k); } };
    } finally {
      entries.drop();
    }
  })()
  inner.value.broadcast.send([]);
}

export function Arc_Inner_observe<T>(self: Arc<Inner<T>>, signal: Signal): void {
  const broadcastId = signal.broadcastId();
  (() => {
    let entries = self.value.entries.write();
    try {
      {
        const _v = entries.value.get(broadcastId);
        if (_v != null) {
          const entry = _v;
          entry.markedForRemoval = false;
          return;
        }
      }
    } finally {
      entries.drop();
    }
  })()
  const weak = self.downgrade();
  let _moved0 = false;
  const guard = signal.listen(Arc.new((_) => {
    {
      const _v1 = weak.upgrade();
      if (_v1 != null) {
        const inner = _v1;
        trigger(inner);
      }
    }
  }));
  try {
    let entries = self.value.entries.write();
    try {
      _moved0 = true;
      entries.value.set(broadcastId, new SubscriptionEntry(guard, false));
    } finally {
      entries.drop();
    }
  } finally {
    if (!_moved0) guard.drop();
  }
}

export function Arc_Inner_observerId<T>(self: Arc<Inner<T>>): number {
  return self.asPtr();
}

export function Arc_Inner_asAny<T>(self: Arc<Inner<T>>): Any {
  return self;
}

