// MIRRORS: ankurah/signals/src/observer/callback_observer.rs
import { Struct, Arc, Weak, RwLock, OwnedClosure } from '@ankurah/base';
import { BroadcastId, ListenerGuard } from '../broadcast';
import { CurrentObserver } from '../context';
import { Observer } from '../observer';
import { Signal } from '../signal';

export class CallbackObserver extends Struct implements Observer {
  _0: Arc<Inner>;

  constructor(_0: Arc<Inner>) {
    super();
    this._0 = _0;
  }

  static new<F extends Fn>(callback: Arc<F>): CallbackObserver {
    return new CallbackObserver(Arc.new(new Inner(new OwnedClosure([callback], () => callback()), new RwLock(new Map()))));
  }

  trigger(): void {
    this.withContext(this._0.value.callback);
  }

  withContext<F extends Fn>(f: F): void {
    this.markAllForRemoval();
    CurrentObserver.set(this.clone());
    f();
    CurrentObserver.remove(this);
    this.sweepMarkedListeners();
  }

  clear(): void {
    const _t0 = this._0.value.entries.write();
    try {
      _t0.value.clear();
    } finally {
      _t0.drop();
    }
  }

  markAllForRemoval(): void {
    let entries = this._0.value.entries.write();
    try {
      for (const entry of entries.value.values()) {
        entry.markedForRemoval = true;
      }
    } finally {
      entries.drop();
    }
  }

  sweepMarkedListeners(): void {
    let entries = this._0.value.entries.write();
    try {
      { for (const [_k, _v] of entries.value) { if (!((_, entry) => !entry.markedForRemoval(_k, _v))) entries.value.delete(_k); } };
    } finally {
      entries.drop();
    }
  }

  observe(signal: Signal): void {
    const broadcastId = signal.broadcastId();
    let entries = this._0.value.entries.write();
    try {
      {
        const _v = entries.value.get(broadcastId);
        if (_v != null) {
          const entry = _v;
          entry.markedForRemoval = false;
          return;
        }
      }
      const weak = new WeakCallbackObserver(this._0.downgrade());
      entries.value.set(broadcastId, new SubscriptionEntry(signal.listen(Arc.new((_) => {
        {
          const _v1 = weak.upgrade();
          if (_v1 != null) {
            const observer = _v1;
            observer.trigger();
          }
        }
      })), false));
    } finally {
      entries.drop();
    }
  }

  observerId(): number {
    return this._0.asPtr() as unknown as number;
  }

  asAny(): Any {
    return this;
  }

  clone(): CallbackObserver {
    return new CallbackObserver(this._0.clone());
  }
}

class SubscriptionEntry extends Struct {
  _guard: ListenerGuard;
  markedForRemoval: boolean;

  constructor(_guard: ListenerGuard, markedForRemoval: boolean) {
    super();
    this._guard = _guard;
    this.markedForRemoval = markedForRemoval;
  }
}

class Inner extends Struct {
  callback: () => void;
  entries: RwLock<Map<BroadcastId, SubscriptionEntry>>;

  constructor(callback: () => void, entries: RwLock<Map<BroadcastId, SubscriptionEntry>>) {
    super();
    this.callback = callback;
    this.entries = entries;
  }
}

class WeakCallbackObserver extends Struct {
  _0: Weak<Inner>;

  constructor(_0: Weak<Inner>) {
    super();
    this._0 = _0;
  }

  upgrade(): CallbackObserver | null {
    return this._0.upgrade() != null ? (CallbackObserver)(this._0.upgrade()!) : null;
  }
}

