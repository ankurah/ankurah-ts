// MIRRORS: ankurah/signals/src/signal.rs
import { Struct, Arc } from '@ankurah/base';
import { BroadcastId, TListenerGuard } from './broadcast';
export * from './signal/calculated';
export * from './signal/map';
export * from './signal/memo';
export * from './signal/mutable';
export * from './signal/read';

export class ListenerGuard extends Struct {
  _0: TListenerGuard;

  constructor(_0: TListenerGuard) {
    super();
    this._0 = _0;
  }

  static new<T>(guard: ListenerGuard<T>): ListenerGuard {
    return new ListenerGuard(guard);
  }

  broadcastId(): BroadcastId {
    return this._0.broadcastId();
  }

  static from<T>(guard: ListenerGuard<T>): ListenerGuard {
    return new ListenerGuard(guard);
  }
}

export interface Signal {
  listen(listener: Listener): ListenerGuard;
  broadcastId(): BroadcastId;
}

export interface Get<T> {
  get(): T;
}

export interface With<T> {
  with(f: (arg0: T) => R): R;
}

export interface Peek<T> {
  peek(): T;
}

export interface GetReadCell<T> {
  getReadcell(): ReadValueCell<T>;
}

export type Listener = Arc<(arg0: void) => void>;

