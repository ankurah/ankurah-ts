// MIRRORS: ankurah/signals/src/signal/read.rs
import { Struct, Result, Arc } from '@ankurah/base';
import { Broadcast, BroadcastId, ListenerGuard } from '../broadcast';
import { SubscriptionGuard } from '../porcelain/subscribe';
import { Memo } from './memo';
import { ReadValueCell, ValueCell } from '../value';

export class Read<T> extends Struct implements Get, Peek, With, GetReadCell, Signal, Subscribe {
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
    return new Map(this.clone(), transform);
  }

  memo<Output, Transform>(transform: Transform): Memo<Read<T>, T, Output, Transform> {
    return new Memo(this.clone(), transform);
  }

  clone(): Read<T> {
    return new Read(this.value.clone(), this.broadcast.clone());
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
}

