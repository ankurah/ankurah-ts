// MIRRORS: ankurah/core/src/livequery.rs
import { Struct, Drop, Result, Arc, Weak, OwnedClosure, dropOwned, tracing, checkedAdd, wrappingAdd, iterFirst, Notify, tokio, spawn } from '@ankurah/base';
import { CollectionId, Attested, EntityId, Event, QueryId } from '@ankurah/proto';
import { BroadcastId, CurrentObserver, Get, Listener, ListenerGuard, Mut, Peek, Read, Signal, Subscribe, SubscriptionGuard } from '@ankurah/signals';
import { ChangeSet, ItemChange } from './changes';
import { Entity } from './entity';
import { RetrievalError } from './error';
import { View } from './indexel';
import { ContextData, MatchArgs, Node, TNodeErased } from './node';
import { RemoteQuerySubscriber } from './peer_subscription/client_relay';
import { PreNotifyHook } from './reactor';
import { GapFetcher, QueryGapFetcher } from './reactor/fetch_gap';
import { ReactorSubscription } from './reactor/subscription';
import { ReactorUpdate } from './reactor/update';
import { EntityResultSet, ResultSet } from './resultset';
import { spawn } from './task';
import { Selection } from '@ankurah/ankql';

export class EntityLiveQuery extends Struct implements PreNotifyHook {
  _0: Arc<Inner>;

  constructor(_0: Arc<Inner>) {
    super();
    this._0 = _0;
  }

  static new<SE, PA>(node: Node<SE, PA>, collectionId: CollectionId, args: MatchArgs, cdata: ContextData): Result<EntityLiveQuery, RetrievalError> {
    try {
      try {
        const _r0 = node.deref().value.policyAgent.canAccessCollection(cdata, collectionId);
        if (_r0.isErr()) return Result.Err(RetrievalError.fromAccessDenied(_r0.unwrapErr()));
        _r0.drop();
        const _r2 = node.deref().value.policyAgent.filterPredicate(cdata, collectionId, args.selection.takeField('predicate'));
        if (_r2.isErr()) return Result.Err(RetrievalError.fromAccessDenied(_r2.unwrapErr()));
        const _a1 = _r2.unwrap();
        args.selection.predicate.drop();
        args.selection.predicate = _a1;
        const _a3 = node.deref().value.typeResolver.resolveSelectionTypes(args.takeField('selection'));
        args.selection.drop();
        args.selection = _a3;
        let _moved4 = false;
        const subscription = node.deref().value.reactor.subscribe();
        try {
          const resultset = EntityResultSet.empty();
          try {
            const queryId = QueryId.new();
            let _moved5 = false;
            const gapFetcher = Arc.new(QueryGapFetcher.new(node, cdata.clone()));
            try {
              const _b6 = node.clone();
              let _moved8 = false;
              const _b7 = resultset.clone();
              try {
                const _b9 = Mut.new(null);
                let _moved11 = false;
                const _b10 = tokio.sync.Notify.new();
                try {
                  const _b12 = 0;
                  const _b13 = 1;
                  const _b14 = Mut.new([args.selection.clone(), 1]);
                  const _b15 = collectionId.clone();
                  _moved8 = true;
                  _moved11 = true;
                  _moved4 = true;
                  _moved5 = true;
                  const me = new EntityLiveQuery(Arc.new(new Inner(queryId, _b6, subscription, _b7, _b9, _b10, _b12, _b13, _b14, _b15, gapFetcher)));
                  const hasRelay = (node.deref().value.subscriptionRelay != null);
                  if (args.cached || !hasRelay) {
                    const me2 = me.clone();
                    tracing.debug(`LiveQuery::new() spawning initialization task for durable node predicate ${queryId}`);
                    spawn((async () => {
                      tracing.debug(`LiveQuery initialization task starting for predicate ${queryId}`);
                      {
                        const _v = await me2.activate(1);
                        if (_v.isErr()) {
                          const e = _v.unwrapErr();
                          tracing.debug(`LiveQuery initialization failed for predicate ${queryId}: ${e}`);
                          me2._0.error.set(e);
                        } else {
                        tracing.debug(`LiveQuery initialization completed for predicate ${queryId}`);
                      }
                      }
                    })());
                  }
                  if (hasRelay) {
                    node.subscribeRemoteQuery(queryId, collectionId.clone(), args.selection.clone(), cdata.clone(), 1, me.weak());
                  }
                  return Result.Ok(me);
                } finally {
                  if (!_moved11) dropOwned(_b10);
                }
              } finally {
                if (!_moved8) dropOwned(_b7);
              }
            } finally {
              if (!_moved5) gapFetcher.drop();
            }
          } finally {
            resultset.drop();
          }
        } finally {
          if (!_moved4) subscription.drop();
        }
      } finally {
        args.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  map<R extends View>(): LiveQuery<R> {
    return new LiveQuery(this, undefined /* PhantomData */);
  }

  async waitInitialized(): Promise<void> {
    if (this._0.value.initializedVersion >= this._0.value.currentVersion) {
      return;
    }
    await this._0.value.initialized.notified();
  }

  updateSelection(newSelection: TryInto): Result<void, RetrievalError> {
    const _r0 = newSelection.tryInto().mapErr((e) => e);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const newSelection_1 = _r0.unwrap();
    try {
      const newVersion = checkedAdd((() => { const _v = this._0.value.currentVersion; this._0.value.currentVersion = wrappingAdd(this._0.value.currentVersion, 1, 'u32'); return _v; })(), 1, 'u32');
      this._0.value.resultset.setLoaded(false);
      this._0.value.selection.set([newSelection_1.clone(), newVersion]);
      const hasRelay = this._0.value.node.hasSubscriptionRelay();
      if (hasRelay) {
        const _r1 = this._0.value.node.updateRemoteQuery(this._0.value.queryId, newSelection_1.clone(), newVersion);
        if (_r1.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r1.unwrapErr()));
        _r1.drop();
      } else {
        const me2 = this.clone();
        try {
          const queryId = this._0.value.queryId;
          spawn((async () => {
            {
              const _v = await me2.activate(newVersion);
              if (_v.isErr()) {
                const e = _v.unwrapErr();
                tracing.error(`LiveQuery update failed for predicate ${queryId}: ${e}`);
                me2._0.value.error.set(e);
              } else {
              _v.drop();
            }
            }
          })());
        } finally {
          me2.drop();
        }
      }
      return Result.Ok([]);
    } finally {
      newSelection_1.drop();
    }
  }

  async updateSelectionWait(newSelection: TryInto): Promise<Result<void, RetrievalError>> {
    const _r0 = this.updateSelection(newSelection);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    await this.waitInitialized();
    return Result.Ok([]);
  }

  async activate(version: number): Promise<Result<void, RetrievalError>> {
    const [selection, storedVersion] = this._0.value.selection.value();
    if (version < storedVersion) {
      tracing.warn(`LiveQuery - Dropped stale activation request for version ${version} (current version is ${storedVersion})`);
      return Result.Ok([]);
    }
    tracing.debug(`LiveQuery.activate() for predicate ${this._0.value.queryId} (version ${version})`);
    const reactor = this._0.value.node.reactor();
    const initializedVersion = this._0.value.initializedVersion;
    if (initializedVersion === 0) {
      const _r0 = await reactor.addQueryAndNotify(this._0.value.subscription.id(), this._0.value.queryId, this._0.value.collectionId.clone(), selection, this._0.value.node, this._0.value.resultset.clone(), this._0.value.gapFetcher.clone(), this);
      if (_r0.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r0.unwrapErr()));
      _r0.unwrap();
    } else {
      const _r1 = await reactor.updateQueryAndNotify(this._0.value.subscription.id(), this._0.value.queryId, this._0.value.collectionId.clone(), selection, this._0.value.node, version, this);
      if (_r1.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r1.unwrapErr()));
      _r1.drop();
    };
    return Result.Ok([]);
  }

  error(): Read<RetrievalError | null> {
    return this._0.value.error.read();
  }

  queryId(): QueryId {
    return this._0.value.queryId;
  }

  selection(): Read<[Selection, number]> {
    return this._0.value.selection.read();
  }

  weak(): WeakEntityLiveQuery {
    return new WeakEntityLiveQuery(this._0.downgrade());
  }

  markInitialized(version: number): void {
    this._0.value.initializedVersion = version;
    this._0.value.initialized.notifyWaiters();
  }

  preNotify(version: number): void {
    this.markInitialized(version);
  }

  clone(): EntityLiveQuery {
    return new EntityLiveQuery(this._0.clone());
  }
}

class Inner extends Drop {
  queryId: QueryId;
  node: TNodeErased;
  subscription: ReactorSubscription<Entity, Attested<Event>>;
  resultset: EntityResultSet<Entity>;
  error: Mut<RetrievalError | null>;
  initialized: Notify;
  initializedVersion: number;
  currentVersion: number;
  selection: Mut<[Selection, number]>;
  collectionId: CollectionId;
  gapFetcher: Arc<GapFetcher>;

  constructor(queryId: QueryId, node: TNodeErased, subscription: ReactorSubscription<Entity, Attested<Event>>, resultset: EntityResultSet<Entity>, error: Mut<RetrievalError | null>, initialized: Notify, initializedVersion: number, currentVersion: number, selection: Mut<[Selection, number]>, collectionId: CollectionId, gapFetcher: Arc<GapFetcher>) {
    super();
    this.queryId = queryId;
    this.node = node;
    this.subscription = subscription;
    this.resultset = resultset;
    this.error = error;
    this.initialized = initialized;
    this.initializedVersion = initializedVersion;
    this.currentVersion = currentVersion;
    this.selection = selection;
    this.collectionId = collectionId;
    this.gapFetcher = gapFetcher;
  }

  protected override onDrop(): void {
    this.node.unsubscribeRemotePredicate(this.queryId);
  }
}

export class WeakEntityLiveQuery extends Struct implements RemoteQuerySubscriber {
  _0: Weak<Inner>;

  constructor(_0: Weak<Inner>) {
    super();
    this._0 = _0;
  }

  upgrade(): EntityLiveQuery | null {
    const _m0 = this._0.upgrade();
    return (_m0 != null ? (EntityLiveQuery)(_m0!) : null);
  }

  clone(): WeakEntityLiveQuery {
    return new WeakEntityLiveQuery(this._0.clone());
  }

  async subscriptionEstablished(version: number): Promise<void> {
    {
      const _v1 = this.upgrade();
      if (_v1 != null) {
        const livequery = _v1;
        try {
          tracing.debug(`Subscription established for query ${livequery._0.value.queryId}: ${version}`);
          {
            const _v = await livequery.activate(version);
            if (_v.isErr()) {
              const e = _v.unwrapErr();
              tracing.error(`Failed to activate subscription for query ${livequery._0.value.queryId}: ${e}`);
              livequery._0.value.error.set(e);
            } else {
            _v.drop();
          }
          }
        } finally {
          livequery.drop();
        }
      }
    }
  }

  setLastError(error: RetrievalError): void {
    let _moved0 = false;
    try {
      {
        const _v = this.upgrade();
        if (_v != null) {
          const livequery = _v;
          try {
            tracing.info(`Setting last error for LiveQuery ${livequery._0.value.queryId}: ${error}`);
            _moved0 = true;
            livequery._0.value.error.set(error);
          } finally {
            livequery.drop();
          }
        }
      }
    } finally {
      if (!_moved0) error.drop();
    }
  }
}

export class LiveQuery<R extends View & Clone> extends Struct implements Signal, Get<R[]>, Peek<R[]>, Subscribe<ChangeSet<R>> {
  _0: EntityLiveQuery;

  constructor(_0: EntityLiveQuery) {
    super();
    this._0 = _0;
  }

  async waitInitialized(): Promise<void> {
    await this._0.waitInitialized();
  }

  resultset(): ResultSet<R> {
    return this._0._0.value.resultset.wrap();
  }

  loaded(): boolean {
    return this._0._0.value.resultset.isLoaded();
  }

  ids(): EntityId[] {
    return this._0._0.value.resultset.keys();
  }

  idsSorted(): EntityId[] {
    return this._0._0.value.resultset.keys().sorted();
  }

  deref(): EntityLiveQuery {
    return this._0;
  }

  listen(listener: Listener): ListenerGuard {
    return this._0._0.value.subscription.listen(listener);
  }

  broadcastId(): BroadcastId {
    return this._0._0.value.subscription.broadcastId();
  }

  get(): R[] {
    CurrentObserver.track(this);
    const _t0 = this._0._0.value.resultset.wrap();
    try {
      return _t0.peek();
    } finally {
      _t0.drop();
    }
  }

  peek(): R[] {
    const _t0 = this._0._0.value.resultset.wrap();
    try {
      return _t0.peek();
    } finally {
      _t0.drop();
    }
  }

  subscribe<L>(listener: L): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const me = this.clone();
    return this._0._0.value.subscription.subscribe(new OwnedClosure([me, listener_1], (reactorUpdate: ReactorUpdate<Entity, Attested<Event>>) => {
      const changeset = livequeryChangeSetFrom(me._0._0.value.resultset.wrap(), reactorUpdate);
      listener_1(changeset);
    }));
  }

  clone(): LiveQuery<R> {
    return new LiveQuery(this._0.clone(), this._1.clone());
  }
}

function livequeryChangeSetFrom<R extends View>(resultset: ResultSet<R>, reactorUpdate: ReactorUpdate<Entity, Attested<Event>>): ChangeSet<R> {
  try {
    let changes = [];
    const _seq0 = reactorUpdate.items;
    let _at1 = 0;
    try {
      while (_at1 < _seq0.length) {
        const item = _seq0[_at1++];
        try {
          const view = R.fromEntity(item.takeField('entity'));
          {
            const _v = iterFirst(item.predicateRelevance);
            if (_v != null) {
              const [, membershipChange] = _v;
              return membershipChange.match({
                Initial: () => {
                  changes.push(new ItemChange('Initial', { item: view }));
                },
                Add: () => {
                  changes.push(new ItemChange('Add', { item: view, events: item.events }));
                },
                Remove: () => {
                  changes.push(new ItemChange('Remove', { item: view, events: item.events }));
                },
              });
            } else {
            changes.push(new ItemChange('Update', { item: view, events: item.events }));
          }
          }
        } finally {
          item.drop();
        }
      }
    } finally {
      dropOwned(_seq0.slice(_at1));
    }
    return new ChangeSet(resultset, changes);
  } finally {
    reactorUpdate.drop();
  }
}

