// MIRRORS: ankurah/signals/src/broadcast.rs
import { Struct, Enum, Drop, Result, Arc, Weak, RwLock } from '@ankurah/base';

export class BroadcastId extends Struct {
  _0: number;

  constructor(_0: number) {
    super();
    this._0 = _0;
  }

  toString(): string {
    return `${this._0}`;
  }

  equals(other: BroadcastId): boolean {
    if (this._0 !== other._0) return false;
    return true;
  }

  compareTo(other: BroadcastId): number {
    throw new Error('TODO');
  }

  clone(): BroadcastId {
    return new BroadcastId(this._0);
  }
}

export class Broadcast<T extends Clone = void> extends Struct {
  _0: Arc<Inner<T>>;

  constructor(_0: Arc<Inner<T>>) {
    super();
    this._0 = _0;
  }

  static new<T>(): Broadcast<T> {
    return new Broadcast(Arc.new(new Inner(new RwLock(new Map()), 0)));
  }

  id(): BroadcastId {
    return new BroadcastId(this._0.asPtr() as number);
  }

  send(value: T): void {
    const subscribers = (() => {
      const listeners = this._0.value.listeners.read();
      try {
        return [...listeners.value.values()];
      } finally {
        listeners.drop();
      }
    })();
    {
      const _v = subscribers.splitLast();
      if (_v != null) {
        const [last, rest] = _v;
        for (const callback of rest) {
          return callback.match({
            Payload: (v) => {
              const callback = v._0;
              return callback(value.clone());
            },
            NotifyOnly: (v) => {
              const callback = v._0;
              return callback();
            },
          });
        }
        return last.match({
          Payload: (v) => {
            const callback = v._0;
            return callback(value);
          },
          NotifyOnly: (v) => {
            const callback = v._0;
            return callback();
          },
        });
      }
    }
  }

  reference(): Ref<T> {
    return new Ref(this);
  }

  toString(): Result {
    const _t0 = this._0.value.listeners.read();
    try {
      return f.debugStruct('Broadcast').field('listeners', _t0.value.size).finish();
    } finally {
      _t0.drop();
    }
  }

  static default<T>(): Broadcast<T> {
    return Broadcast.new();
  }

  clone(): Broadcast<T> {
    return new Broadcast(this._0.clone());
  }
}

class Inner<T> extends Struct {
  listeners: RwLock<Map<number, BroadcastListener<T>>>;
  nextId: number;

  constructor(listeners: RwLock<Map<number, BroadcastListener<T>>>, nextId: number) {
    super();
    this.listeners = listeners;
    this.nextId = nextId;
  }
}

export class Ref<T> extends Struct {
  _0: Broadcast<T>;

  constructor(_0: Broadcast<T>) {
    super();
    this._0 = _0;
  }

  // A `&T` field is a borrow: dropping this releases the borrow and nothing
  // else, so the cascade must not walk it.
  protected override ownedFields(): unknown[] {
    return [];
  }

  listen<L>(listener: L): ListenerGuard<T> {
    const id = (() => { const _v = this._0._0.value.nextId; this._0._0.value.nextId += 1; return _v; })();
    const _t0 = this._0._0.value.listeners.write();
    try {
      _t0.value.set(id, listener.intoBroadcastListener());
    } finally {
      _t0.drop();
    }
    return new ListenerGuard(this._0._0.downgrade(), id);
  }

  broadcastId(): BroadcastId {
    return new BroadcastId(this._0._0.asPtr() as number);
  }
}

export class ListenerGuard<T = void> extends Drop implements TListenerGuard {
  inner: Weak<Inner<T>>;
  id: number;

  constructor(inner: Weak<Inner<T>>, id: number) {
    super();
    this.inner = inner;
    this.id = id;
  }

  broadcastId(): BroadcastId {
    return new BroadcastId(this.inner.asPtr() as number);
  }

  protected override onDrop(): void {
    {
      const _v = this.inner.upgrade();
      if (_v != null) {
        const inner = _v;
        try {
          const _t0 = inner.value.listeners.write();
          try {
            _t0.value.delete(this.id);
          } finally {
            _t0.drop();
          }
        } finally {
          inner.drop();
        }
      }
    }
  }
}

export type BroadcastListenerV = {
  Payload: { _0: Arc<(arg0: T) => void> };
  NotifyOnly: { _0: Arc<() => void> };
};

export class BroadcastListener<T = void> extends Enum<BroadcastListenerV> {

  intoBroadcastListener(): BroadcastListener<T> {
    return this;
  }

  clone(): BroadcastListener<T> {
    return this.match({
      Payload: (v) => new BroadcastListener('Payload', { _0: v._0.clone() }),
      NotifyOnly: (v) => new BroadcastListener('NotifyOnly', { _0: v._0.clone() }),
    });
  }
}

export interface IntoBroadcastListener<T> {
  intoBroadcastListener(): BroadcastListener<T>;
}

export interface TListenerGuard {
  broadcastId(): BroadcastId;
}

