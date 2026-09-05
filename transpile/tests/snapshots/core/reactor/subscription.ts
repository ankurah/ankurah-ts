// MIRRORS: ankurah/core/src/reactor/subscription.rs
import { Struct, Drop, Result, Arc, OwnedClosure, HashMap, HashSet } from '@ankurah/base';
import { Broadcast, IntoSubscribeListener, ListenerGuard, Signal, Subscribe, SubscriptionGuard } from '@ankurah/signals';
import { SubscriptionError } from '../error';
import { Reactor } from '../reactor';
import { ReactorUpdate } from './update';
import { EntityId, QueryId } from '@ankurah/proto';
import { Broadcast, BroadcastId, BroadcastListener, ListenerGuard, Signal, Subscribe, SubscriptionGuard } from '@ankurah/signals';

export class ReactorSubscriptionId extends Struct {
  _0: Ulid;

  constructor(_0: Ulid) {
    super();
    this._0 = _0;
  }

  static new(): ReactorSubscriptionId {
    return new ReactorSubscriptionId(Ulid.new());
  }

  static default(): ReactorSubscriptionId {
    return ReactorSubscriptionId.new();
  }

  toString(): string {
    return `RS-${this._0}`;
  }

  equals(other: ReactorSubscriptionId): boolean {
    if (!this._0.equals(other._0)) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [this._0.hash()].join('|');
  }

  compareTo(other: ReactorSubscriptionId): number {
    let c = this._0.compareTo(other._0);
    if (c !== 0) return c;
    return 0;
  }

  clone(): ReactorSubscriptionId {
    return new ReactorSubscriptionId(this._0.clone());
  }

  debug(): string {
    return `ReactorSubscriptionId(${this._0})`;
  }
}

class ReactorSubInner<E extends AbstractEntity & Filterable, Ev extends Clone> extends Drop {
  subscriptionId: ReactorSubscriptionId;
  reactor: Reactor<E, Ev>;
  broadcast: Broadcast<ReactorUpdate<E, Ev>>;

  constructor(subscriptionId: ReactorSubscriptionId, reactor: Reactor<E, Ev>, broadcast: Broadcast<ReactorUpdate<E, Ev>>) {
    super();
    this.subscriptionId = subscriptionId;
    this.reactor = reactor;
    this.broadcast = broadcast;
  }

  protected override onDrop(): void {
    const _ = this.reactor.unsubscribe(this.subscriptionId);
  }
}

export class ReactorSubscription<E extends AbstractEntity & Filterable = Entity, Ev extends Clone = Attested<Event>> extends Struct implements Subscribe<ReactorUpdate<E, Ev>>, Signal {
  _0: Arc<ReactorSubInner<E, Ev>>;

  constructor(_0: Arc<ReactorSubInner<E, Ev>>) {
    super();
    this._0 = _0;
  }

  id(): ReactorSubscriptionId {
    return this._0.value.subscriptionId;
  }

  removePredicate(queryId: QueryId): Result<void, SubscriptionError> {
    const _r0 = this._0.value.reactor.removeQuery(this._0.value.subscriptionId, queryId);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    return Result.Ok([]);
  }

  addEntitySubscriptions(entityIds: EntityId[]): void {
    const entityIds_1 = [...entityIds];
    this._0.value.reactor.addEntitySubscriptions(this._0.value.subscriptionId, entityIds_1);
  }

  removeEntitySubscriptions(entityIds: EntityId[]): void {
    const entityIds_1 = [...entityIds];
    this._0.value.reactor.removeEntitySubscriptions(this._0.value.subscriptionId, entityIds_1);
  }

  clone(): ReactorSubscription<E, Ev> {
    return new ReactorSubscription(this._0.clone());
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const _t0 = this._0.value.broadcast.reference();
    try {
      const guard = _t0.listen(listener_1);
      return SubscriptionGuard.new(guard);
    } finally {
      _t0.drop();
    }
  }

  listen(listener: Listener): ListenerGuard {
    const _t0 = this._0.value.broadcast.reference();
    try {
      return _t0.listen(new BroadcastListener('NotifyOnly', { _0: Arc.new(new OwnedClosure([listener], () => listener([]))) }));
    } finally {
      _t0.drop();
    }
  }

  broadcastId(): BroadcastId {
    return this._0.value.broadcast.id();
  }
}

