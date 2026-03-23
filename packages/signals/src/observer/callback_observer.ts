// MIRRORS: ankurah/signals/src/observer/callback_observer.rs
import { Struct, Arc, Weak, RwLock } from '@ankurah/base';
import { BroadcastId, ListenerGuard } from '../broadcast';
import { CurrentObserver } from '../context';
import { Signal } from '../signal';

export class CallbackObserver extends Struct implements Observer {
  _0: Arc<Inner>;

  constructor(_0: Arc<Inner>) {
    super();
    this._0 = _0;
  }

  static new<F extends Fn>(callback: Arc<F>): CallbackObserver {
    return new CallbackObserver(Arc.new(new Inner(() => callback(), new RwLock(new Map()))));
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
    this._0.value.entries.write().value.expect('entries lock is poisoned').clear();
  }

  markAllForRemoval(): void {
    let entries = this._0.value.entries.write().value.expect('entries lock is poisoned');
    for (const entry of entries.valuesMut()) {
      entry.markedForRemoval = true;
    }
    entries.drop();

  }

  sweepMarkedListeners(): void {
    let entries = this._0.value.entries.write().value.expect('entries lock is poisoned');
    entries.retain((_, entry) => !entry.markedForRemoval);
    entries.drop();
  }

  observe(signal: Signal): void {
    const broadcastId = signal.broadcastId();
    let entries = this._0.value.entries.write().value.expect('entries lock is poisoned');
    if (entries.getMut(broadcastId) != null) {
      const entry = entries.getMut(broadcastId);
      entry.markedForRemoval = false;
      return;
    }
    const weak = new WeakCallbackObserver(this._0.downgrade());
    entries.insert(broadcastId, new SubscriptionEntry(signal.listen(Arc.new((_) => {
      if (weak.upgrade() != null) {
        const observer = weak.upgrade();
        observer.trigger();
      }
    })), false));
    weak.drop();
    entries.drop();
    broadcastId.drop();
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
  Guard: ListenerGuard;
  markedForRemoval: boolean;

  constructor(Guard: ListenerGuard, markedForRemoval: boolean) {
    super();
    this.Guard = Guard;
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

