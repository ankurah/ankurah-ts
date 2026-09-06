// MIRRORS: ankurah/core/src/reactor.rs
import { Struct, Result, Arc, Mutex, AnyhowError, dropOwned, tracing, iterFindMap, iterFilterMap, HashMap, AsyncMutex, tokio } from '@ankurah/base';
import { Entity } from './entity';
import { SubscriptionError } from './error';
import { IndexDirection, IndexKeyPart, KeySpec, NullsOrder } from './indexing/key_spec';
import { ContextData, Node, TNodeErased } from './node';
import { CandidateChanges } from './reactor/candidate_changes';
import { GapFetcher } from './reactor/fetch_gap';
import { ReactorSubInner, ReactorSubscription, ReactorSubscriptionId } from './reactor/subscription';
import { Subscription } from './reactor/subscription_state';
import { WatcherOp, WatcherSet } from './reactor/watcherset';
import { EntityResultSet } from './resultset';
import { ValueType } from './value/index';
import { OrderByItem, Selection } from '@ankurah/ankql';
import { CollectionId, EntityId, QueryId } from '@ankurah/proto';
import { Broadcast } from '@ankurah/signals';
export * from './reactor/fetch_gap';

export class Reactor<E extends AbstractEntity & Filterable = Entity, Ev extends Clone = Attested<Event>> extends Struct {
  _0: Arc<ReactorInner<E, Ev>>;

  constructor(_0: Arc<ReactorInner<E, Ev>>) {
    super();
    this._0 = _0;
  }

  static new<E, Ev>(): Reactor<E, Ev> {
    return new Reactor(Arc.new(new ReactorInner(new Mutex(new HashMap()), Arc.new(new Mutex(WatcherSet.new())), tokio.sync.Mutex.new([]))));
  }

  subscribe(): ReactorSubscription<E, Ev> {
    const broadcast = Broadcast.new();
    const subscription = Subscription.new(broadcast.clone(), this._0.value.watcherSet.clone());
    const subscriptionId = subscription.id();
    const _t0 = this._0.value.subscriptions.lock();
    try {
      _t0.value.set(subscriptionId, subscription);
    } finally {
      _t0.drop();
    }
    return new ReactorSubscription(Arc.new(new ReactorSubInner(subscriptionId, this.clone(), broadcast)));
  }

  unsubscribe(subId: ReactorSubscriptionId): Result<void, SubscriptionError> {
    const _m3 = (() => {
      {
        let subscriptions = this._0.value.subscriptions.lock();
        try {
          const _m0 = subscriptions.value.remove(subId);
          const _m1 = new SubscriptionError('SubscriptionNotFound', {});
          const _r2 = (_m0 != null ? (_m1.drop(), Result.Ok(_m0!)) : Result.Err(_m1));
          if (_r2.isErr()) return { $jump: 'return', $value: Result.Err(_r2.unwrapErr()) };
          return _r2.unwrap();
        } finally {
          subscriptions.drop();
        }
      }
    })();
    if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
    const subscription = (_m3 as any);
    const queries = subscription.takeAllQueries();
    let watcherSet = this._0.value.watcherSet.value.lock();
    try {
      for (const [queryId, queryState] of queries) {
        {
          const _v = queryState.selection;
          if (_v != null) {
            const selection = _v;
            watcherSet.value.recursePredicateWatchers(queryState.collectionId, selection.predicate, [subId, queryId], new WatcherOp('Remove', {}));
          }
        }
        const entityIds = queryState.resultset.keys();
        watcherSet.value.removeEntitySubscriptions(subId, entityIds);
      }
      return Result.Ok([]);
    } finally {
      watcherSet.drop();
    }
  }

  removeQuery(subscriptionId: ReactorSubscriptionId, queryId: QueryId): Result<void, SubscriptionError> {
    const _m3 = (() => {
      {
        const subscriptions = this._0.value.subscriptions.lock();
        try {
          const _m0 = subscriptions.value.get(subscriptionId);
          const _m1 = new SubscriptionError('SubscriptionNotFound', {});
          const _r2 = (_m0 != null ? (_m1.drop(), Result.Ok(_m0!)) : Result.Err(_m1));
          if (_r2.isErr()) return { $jump: 'return', $value: Result.Err(_r2.unwrapErr()) };
          return _r2.unwrap();
        } finally {
          subscriptions.drop();
        }
      }
    })();
    if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
    const subscription = (_m3 as any);
    const _r4 = subscription.removeQuery(queryId).okOr(new SubscriptionError('PredicateNotFound', {}));
    if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
    const queryState = _r4.unwrap();
    {
      const _v = queryState.selection;
      if (_v != null) {
        const selection = _v;
        let watcherSet = this._0.value.watcherSet.value.lock();
        try {
          const watcherId = [subscriptionId, queryId];
          watcherSet.value.recursePredicateWatchers(queryState.collectionId, selection.predicate, watcherId, new WatcherOp('Remove', {}));
        } finally {
          watcherSet.drop();
        }
      }
    }
    return Result.Ok([]);
  }

  addEntitySubscriptions(subscriptionId: ReactorSubscriptionId, entityIds: EntityId[]): void {
    const subscription = (() => {
      const subscriptions = this._0.value.subscriptions.lock();
      try {
        return subscriptions.value.get(subscriptionId);
      } finally {
        subscriptions.drop();
      }
    })();
    {
      const _v = subscription;
      if (_v != null) {
        const subscription = _v;
        let watcherSet = this._0.value.watcherSet.value.lock();
        try {
          for (const entityId of entityIds) {
            subscription.addEntitySubscription(entityId);
            watcherSet.value.addEntitySubscription(subscriptionId, entityId);
          }
        } finally {
          watcherSet.drop();
        }
      }
    }
  }

  removeEntitySubscriptions(subscriptionId: ReactorSubscriptionId, entityIds: EntityId[]): void {
    let subscriptions = this._0.value.subscriptions.lock();
    try {
      let watcherSet = this._0.value.watcherSet.value.lock();
      try {
        {
          const _v = subscriptions.value.get(subscriptionId);
          if (_v != null) {
            const subscription = _v;
            for (const entityId of entityIds) {
              subscription.removeEntitySubscription(entityId);
              const shouldRemove = !subscription.anyQueryMatches(entityId);
              if (shouldRemove) {
                watcherSet.value.removeEntitySubscription(subscriptionId, entityId);
              }
            }
          }
        }
      } finally {
        watcherSet.drop();
      }
    } finally {
      subscriptions.drop();
    }
  }

  async addQueryAndNotify<H extends PreNotifyHook>(subscriptionId: ReactorSubscriptionId, queryId: QueryId, collectionId: CollectionId, selection: Selection, node: TNodeErased, resultset: EntityResultSet<E>, gapFetcher: Arc<GapFetcher>, preNotifyHook: H): Promise<Result<void, Error>> {
    let _moved0 = false;
    try {
      try {
        try {
          try {
            const _m3 = (() => {
              {
                const subscriptions = this._0.value.subscriptions.lock();
                try {
                  const _m1 = subscriptions.value.get(subscriptionId);
                  const _r2 = (_m1 != null ? Result.Ok(_m1!) : Result.Err((() => AnyhowError.msg(`Subscription ${subscriptionId.debug()} not found`))()));
                  if (_r2.isErr()) return { $jump: 'return', $value: Result.Err(_r2.unwrapErr()) };
                  return _r2.unwrap();
                } finally {
                  subscriptions.drop();
                }
              }
            })();
            if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
            const subscription = (_m3 as any);
            const _r4 = await node.fetchEntitiesFromLocal(collectionId, selection);
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            const includedEntities = _r4.unwrap();
            _moved0 = true;
            const _r5 = subscription.registerQuery(queryId, collectionId.clone(), resultset.clone(), gapFetcher);
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            _r5.drop();
            let reactorUpdateItems = [];
            const _r6 = subscription.updateQuery(queryId, collectionId.clone(), selection.clone(), includedEntities, 1, reactorUpdateItems);
            if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
            const _newlyAdded = _r6.unwrap();
            await subscription.fillGapsForQuery(queryId, reactorUpdateItems);
            resultset.setLoaded(true);
            preNotifyHook.preNotify(1);
            subscription.sendUpdate(reactorUpdateItems);
            return Result.Ok([]);
          } finally {
            if (!_moved0) gapFetcher.drop();
          }
        } finally {
          resultset.drop();
        }
      } finally {
        selection.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  async updateQueryAndNotify<H extends PreNotifyHook>(subscriptionId: ReactorSubscriptionId, queryId: QueryId, collectionId: CollectionId, selection: Selection, node: TNodeErased, version: number, preNotifyHook: H): Promise<Result<void, Error>> {
    try {
      try {
        const _r0 = await node.fetchEntitiesFromLocal(collectionId, selection);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const includedEntities = _r0.unwrap();
        const _m3 = (() => {
          {
            const subscriptions = this._0.value.subscriptions.lock();
            try {
              const _m1 = subscriptions.value.get(subscriptionId);
              const _r2 = (_m1 != null ? Result.Ok(_m1!) : Result.Err((() => AnyhowError.msg(`Subscription ${subscriptionId.debug()} not found`))()));
              if (_r2.isErr()) return { $jump: 'return', $value: Result.Err(_r2.unwrapErr()) };
              return _r2.unwrap();
            } finally {
              subscriptions.drop();
            }
          }
        })();
        if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
        const subscription = (_m3 as any);
        let reactorUpdateItems = [];
        const _r4 = subscription.updateQuery(queryId, collectionId.clone(), selection.clone(), includedEntities, version, reactorUpdateItems);
        if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
        const _newlyAdded = _r4.unwrap();
        await subscription.fillGapsForQuery(queryId, reactorUpdateItems);
        preNotifyHook.preNotify(version);
        if (!(reactorUpdateItems.length === 0)) {
          subscription.sendUpdate(reactorUpdateItems);
        }
        return Result.Ok([]);
      } finally {
        selection.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  async notifyChange<C extends ChangeNotification & Clone>(changes: C[]): Promise<void> {
    const _notifyGuard = await this._0.value.notifyLock.lock();
    try {
      const changes_1 = Arc.new(changes);
      try {
        tracing.debug(`Reactor.notify_change(${changes_1.value.length} changes)`);
        let _moved0 = false;
        let candidatesBySub = new HashMap<ReactorSubscriptionId, CandidateChanges<C>>();
        try {
          (() => {
            const watcherSet = this._0.value.watcherSet.value.lock();
            try {
              for (const [offset, change] of [...changes_1.value].entries()) {
                watcherSet.value.accumulateInterestedWatchers(change.entity(), offset, changes_1, candidatesBySub);
              }
            } finally {
              watcherSet.drop();
            }
          })();
          const evaluations = (() => {
            const subscriptions = this._0.value.subscriptions.lock();
            try {
              _moved0 = true;
              return iterFilterMap([...candidatesBySub], ([subId, candidates]) => {
                const _m1 = subscriptions.value.get(subId);
                return (_m1 != null ? ((subscription) => subscription.clone().evaluateChanges(candidates))(_m1!) : null);
              });
            } finally {
              subscriptions.drop();
            }
          })();
          const allWatcherChanges = (await joinAll(evaluations)).intoIter().flatten();
          let watcherSet = this._0.value.watcherSet.value.lock();
          try {
            const _seq2 = allWatcherChanges;
            let _at3 = 0;
            try {
              while (_at3 < _seq2.length) {
                const change = _seq2[_at3++];
                watcherSet.value.applyWatcherChange(change);
              }
            } finally {
              dropOwned(_seq2.slice(_at3));
            }
          } finally {
            watcherSet.drop();
          }
        } finally {
          if (!_moved0) dropOwned(candidatesBySub);
        }
      } finally {
        changes_1.drop();
      }
    } finally {
      _notifyGuard.drop();
    }
  }

  systemReset(): void {
    (() => {
      let watcherSet = this._0.value.watcherSet.value.lock();
      try {
        watcherSet.value.clearEntityWatchers();
      } finally {
        watcherSet.drop();
      }
    })();
    const subscriptions = this._0.value.subscriptions.lock();
    try {
      for (const subscription of subscriptions.value.values()) {
        subscription.systemReset();
      }
    } finally {
      subscriptions.drop();
    }
  }

  async upsertQuery<SE, PA>(subscriptionId: ReactorSubscriptionId, queryId: QueryId, collectionId: CollectionId, selection: Selection, node: Node<SE, PA>, cdata: ContextData, version: number): Promise<Result<Entity[], Error>> {
    try {
      try {
        const _m2 = (() => {
          {
            const subscriptions = this._0.value.subscriptions.lock();
            try {
              const _m0 = subscriptions.value.get(subscriptionId);
              const _r1 = (_m0 != null ? Result.Ok(_m0!) : Result.Err((() => AnyhowError.msg(`Subscription ${subscriptionId.debug()} not found`))()));
              if (_r1.isErr()) return { $jump: 'return', $value: Result.Err(_r1.unwrapErr()) };
              return _r1.unwrap();
            } finally {
              subscriptions.drop();
            }
          }
        })();
        if ((_m2 as any)?.$jump === 'return') return (_m2 as any).$value;
        const subscription = (_m2 as any);
        const _r3 = await node.fetchEntitiesFromLocal(collectionId, selection);
        if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
        let _moved4 = false;
        const includedEntities = _r3.unwrap();
        try {
          const resultset = subscription.upsertQuery(queryId, collectionId.clone(), node, cdata);
          _moved4 = true;
          const _r5 = subscription.updateQuery(queryId, collectionId.clone(), selection.clone(), includedEntities, version, []);
          if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
          let allEntities = _r5.unwrap();
          await subscription.fillGapsForQueryEntities(queryId, allEntities);
          resultset.setLoaded(true);
          return Result.Ok(allEntities);
        } finally {
          if (!_moved4) dropOwned(includedEntities);
        }
      } finally {
        selection.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  clone(): Reactor<E, Ev> {
    return new Reactor(this._0.clone());
  }

  static default<E, Ev>(): Reactor<E, Ev> {
    return Reactor.new();
  }

  toString(): Result {
    let _result = '';
    const watcherSet = this._0.value.watcherSet.value.lock();
    try {
      const subscriptions = this._0.value.subscriptions.lock();
      try {
        const [indexWatchers, wildcardWatchers, entityWatchers] = watcherSet.value.debugData();
        _result += `Reactor { subscriptions: ${subscriptions}, index_watchers: ${`{${Array.from(indexWatchers).map(($p) => `${(($t) => `(${$t[0].debug()}, ${$t[1].debug()})`)($p[0])}: ${$p[1].debug()}`).join(', ')}}`}, wildcard_watchers: ${wildcardWatchers}, entity_watchers: ${entityWatchers} }`;
        return _result;
      } finally {
        subscriptions.drop();
      }
    } finally {
      watcherSet.drop();
    }
  }
}

class ReactorInner<E extends AbstractEntity & Filterable, Ev> extends Struct {
  subscriptions: Mutex<HashMap<ReactorSubscriptionId, Subscription<E, Ev>>>;
  watcherSet: Arc<Mutex<WatcherSet>>;
  notifyLock: AsyncMutex<void>;

  constructor(subscriptions: Mutex<HashMap<ReactorSubscriptionId, Subscription<E, Ev>>>, watcherSet: Arc<Mutex<WatcherSet>>, notifyLock: AsyncMutex<void>) {
    super();
    this.subscriptions = subscriptions;
    this.watcherSet = watcherSet;
    this.notifyLock = notifyLock;
  }
}

export interface AbstractEntity {
  collection(): CollectionId;
  id(): EntityId;
  value(field: string): Value | null;
}

export interface ChangeNotification {
  intoParts(): [Entity, Event[]];
  entity(): Entity;
  events(): Event[];
}

export interface PreNotifyHook {
  preNotify(version: number): void;
}

function buildKeySpecFromSelection<E extends AbstractEntity>(orderBy: OrderByItem[], resultset: EntityResultSet<E>): Result<KeySpec, Error> {
  let keyparts = [];
  const read = resultset.read();
  try {
    for (const item of orderBy) {
      const column = item.path.property();
      const valueType = iterFindMap(read.iterEntities(), ([, e]) => e.value(column).map((v) => ValueType.of(v))) ?? new ValueType('String', {});
      const direction = item.direction.match({
        Asc: () => new IndexDirection('Asc', {}),
        Desc: () => new IndexDirection('Desc', {}),
      });
      keyparts.push(new IndexKeyPart(column, null, direction, valueType, new NullsOrder('Last', {}), null));
    }
    return Result.Ok(new KeySpec(keyparts));
  } finally {
    read.drop();
  }
}

export function Unit_preNotify(self: void, _version: number): void {

}

export function Subscription_toString<E extends AbstractEntity, Ev extends Clone>(self: Subscription<E, Ev>, f: Formatter): Result {
  return `Subscription { id: ${self.id().debug()}, queries: ${self.queriesLen()} }`;
}

