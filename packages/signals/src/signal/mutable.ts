// MIRRORS: ankurah/signals/src/signal/mutable.rs
import { Struct, Arc } from '@ankurah/base';
import { Broadcast, BroadcastId, ListenerGuard } from '../broadcast';
import { SubscriptionGuard } from '../porcelain/subscribe';
import { Read } from './read';
import { ReadValueCell, ValueCell } from '../value';

export class Mut<T> extends Struct implements Get, Peek, With, GetReadCell, Signal, Subscribe {
  value: ValueCell<T>;
  broadcast: Broadcast<void>;

  constructor(value: ValueCell<T>, broadcast: Broadcast<void>) {
    super();
    this.value = value;
    this.broadcast = broadcast;
  }

  static new<T>(value: T): Mut<T> {
    const broadcast = new Broadcast();
    return new Mut(new ValueCell(value), broadcast);
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

  clone(): Mut<T> {
    return new Mut(this.value.clone(), this.broadcast.clone());
  }
}

