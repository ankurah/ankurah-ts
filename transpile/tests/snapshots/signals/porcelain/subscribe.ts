// MIRRORS: ankurah/signals/src/porcelain/subscribe.rs
import { Struct } from '@ankurah/base';
import { ListenerGuard } from '../broadcast';

export class SubscriptionGuard extends Struct {
  Listenerguard: Any;

  constructor(Listenerguard: Any) {
    super();
    this.Listenerguard = Listenerguard;
  }

  static new(lguard: ListenerGuard): SubscriptionGuard {
    return new SubscriptionGuard(lguard);
  }
}

export interface IntoSubscribeListener<T> {
  intoSubscribeListener(): SubscribeListener<T>;
}

export interface Subscribe<T> {
  subscribe(listener: F): SubscriptionGuard;
}

export interface DynSubscribe<T> {
  dynSubscribe(listener: (arg0: T) => void): SubscriptionGuard;
}

export interface GetAndDynSubscribe<T> {
}

export type SubscribeListener = (arg0: T) => void;

