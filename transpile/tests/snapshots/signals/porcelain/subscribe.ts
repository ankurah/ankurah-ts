// MIRRORS: ankurah/signals/src/porcelain/subscribe.rs
import { Struct, OwnedClosure, Sender, UnboundedSender } from '@ankurah/base';
import { ListenerGuard } from '../signal';

export class SubscriptionGuard extends Struct {
  _listenerguard: Any;

  constructor(_listenerguard: Any) {
    super();
    this._listenerguard = _listenerguard;
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

export function dynSubscribe<S extends Subscribe, T>(self: S, listener: (arg0: T) => void): SubscriptionGuard {
  return Subscribe.subscribe(self, listener);
}

export function Sender_intoSubscribeListener<T>(self: Sender<T>): SubscribeListener<T> {
  return (value) => {
    const _ = self.send(value);
  };
}

export function UnboundedSender_intoSubscribeListener<T>(self: UnboundedSender<T>): SubscribeListener<T> {
  try {
    return new OwnedClosure([this], (value) => {
      const _ = self.send(value);
    });
  } finally {
    self.drop();
  }
}

export function intoSubscribeListener<F extends (arg0: T) => void, T>(self: F): SubscribeListener<T> {
  return self;
}

export function IntoSubscribeListener_dispatch_intoSubscribeListener<T>(self: unknown): (arg0: T) => void {
  if (self instanceof Sender) return Sender_intoSubscribeListener(self as any);
  if (self instanceof UnboundedSender) return UnboundedSender_intoSubscribeListener(self as any);
  if (typeof self === 'function' || self instanceof OwnedClosure) return intoSubscribeListener(self as any);
  throw new Error(`BUG: no IntoSubscribeListener impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

