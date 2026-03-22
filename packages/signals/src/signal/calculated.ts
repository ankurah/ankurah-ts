// MIRRORS: ankurah/signals/src/signal/calculated.rs
import { Struct, Arc, RwLock } from '@ankurah/base';
import { Broadcast, BroadcastId, ListenerGuard } from '../broadcast';
import { SubscriptionGuard } from '../porcelain/subscribe';
import { Signal } from '../signal';
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
    const inner = Arc.new(new Inner(compute, new ValueCell(null), new Broadcast(), new RwLock(new Map())));
    trigger(inner);
    return new Calculated(inner);
  }

  clone(): Calculated<T> {
    return new Calculated(Arc.clone(this._0));
  }
}

function trigger(inner: Arc<Inner<T>>): void {
  (() => {
    let entries = inner.entries.write();
    for (const entry of entries.valuesMut()) {
      entry.markedForRemoval = true;
    }
    entries.drop();

  })()
  CurrentObserver.set(Arc.clone(inner));
  const newValue = (inner.compute)();
  inner.value.set(newValue);
  CurrentObserver.remove(inner);
  (() => {
    let entries = inner.entries.write();
    entries.retain((_, entry) => !entry.markedForRemoval);
    entries.drop();
  })()
  inner.broadcast.send([]);
  newValue.drop();
}

