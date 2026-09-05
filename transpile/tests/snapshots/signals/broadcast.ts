// MIRRORS: ankurah/signals/src/broadcast.rs
import { Struct, Enum, Drop, Result, Arc, Weak, RwLock, OwnedClosure, wrappingAdd, HashMap, HashSet, keyHash, Sender, UnboundedSender } from '@ankurah/base';

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

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this._0)].map((p) => p.length + ':' + p).join('');
  }

  compareTo(other: BroadcastId): number {
    let c = this._0 < other._0 ? -1 : this._0 > other._0 ? 1 : 0;
    if (c !== 0) return c;
    return 0;
  }

  clone(): BroadcastId {
    return new BroadcastId(this._0);
  }

  debug(): string {
    return `BroadcastId(${String(this._0)})`;
  }
}

export class Broadcast<T extends Clone = void> extends Struct {
  _0: Arc<Inner<T>>;

  constructor(_0: Arc<Inner<T>>) {
    super();
    this._0 = _0;
  }

  static new<T>(): Broadcast<T> {
    return new Broadcast(Arc.new(new Inner(new RwLock(new HashMap()), 0)));
  }

  id(): BroadcastId {
    return new BroadcastId(this._0.asPtr());
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
          callback.match({
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
  listeners: RwLock<HashMap<number, BroadcastListener<T>>>;
  nextId: number;

  constructor(listeners: RwLock<HashMap<number, BroadcastListener<T>>>, nextId: number) {
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
    const id = (() => { const _v = this._0._0.value.nextId; this._0._0.value.nextId = wrappingAdd(this._0._0.value.nextId, 1, 'usize'); return _v; })();
    const _t0 = this._0._0.value.listeners.write();
    try {
      _t0.value.set(id, IntoBroadcastListener_dispatch_intoBroadcastListener(listener));
    } finally {
      _t0.drop();
    }
    return new ListenerGuard(this._0._0.downgrade(), id);
  }

  broadcastId(): BroadcastId {
    return new BroadcastId(this._0._0.asPtr());
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
    return new BroadcastId(this.inner.asPtr());
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

export type BroadcastListenerV<T = void> = {
  Payload: { _0: Arc<(arg0: T) => void> };
  NotifyOnly: { _0: Arc<() => void> };
};

export class BroadcastListener<T = void> extends Enum<BroadcastListenerV<T>> {

  intoBroadcastListener(): BroadcastListener<T> {
    return this;
  }

  clone(): BroadcastListener<T> {
    return this.match({
      Payload: (v) => new BroadcastListener<T>('Payload', { _0: v._0.clone() }),
      NotifyOnly: (v) => new BroadcastListener<T>('NotifyOnly', { _0: v._0.clone() }),
    });
  }
}

export interface IntoBroadcastListener<T> {
  intoBroadcastListener(): BroadcastListener<T>;
}

export interface TListenerGuard {
  broadcastId(): BroadcastId;
}

export function intoBroadcastListener<F extends (arg0: T) => void, T>(self: F): BroadcastListener<T> {
  return new BroadcastListener('Payload', { _0: Arc.new(self) });
}

export function Arc_Fn1_intoBroadcastListener<T>(self: Arc<(arg0: T) => void>): BroadcastListener<T> {
  return new BroadcastListener('Payload', { _0: self });
}

export function Arc_Fn0_intoBroadcastListener<T>(self: Arc<() => void>): BroadcastListener<T> {
  return new BroadcastListener('NotifyOnly', { _0: self });
}

export function UnboundedSender_intoBroadcastListener<T>(self: UnboundedSender<T>): BroadcastListener<T> {
  return new BroadcastListener('Payload', { _0: Arc.new(new OwnedClosure([this], (value) => {
    const _ = self.send(value);
  })) });
}

export function Sender_intoBroadcastListener<T>(self: Sender<T>): BroadcastListener<T> {
  return new BroadcastListener('Payload', { _0: Arc.new((value) => {
    const _ = self.send(value);
  }) });
}

export function IntoBroadcastListener_dispatch_intoBroadcastListener<T>(self: unknown): BroadcastListener<T> {
  if (typeof self === 'function' || self instanceof OwnedClosure) return intoBroadcastListener(self as any);
  if (self instanceof BroadcastListener) return (self as any).intoBroadcastListener();
  if (self instanceof Arc && ((typeof self.value === 'function' && self.value.length === 1) || (self.value instanceof OwnedClosure && self.value.$arity === 1))) return Arc_Fn1_intoBroadcastListener(self as any);
  if (self instanceof Arc && ((typeof self.value === 'function' && self.value.length === 0) || (self.value instanceof OwnedClosure && self.value.$arity === 0))) return Arc_Fn0_intoBroadcastListener(self as any);
  if (self instanceof UnboundedSender) return UnboundedSender_intoBroadcastListener(self as any);
  if (self instanceof Sender) return Sender_intoBroadcastListener(self as any);
  throw new Error(`BUG: no IntoBroadcastListener impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

