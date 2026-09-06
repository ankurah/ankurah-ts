// MIRRORS: ankurah/core/src/reactor/subscription_state.rs
import { Struct, Result, Arc, Mutex, AnyhowError, dropOwned, derivedClone, valueNotEquals, tracing, checkedSub, iterFilterMap, HashMap, HashSet, spawn } from '@ankurah/base';
import { Entity } from '../entity';
import { ContextData, Node } from '../node';
import { AbstractEntity, ChangeNotification } from '../reactor';
import { EntityResultSet } from '../resultset';
import { evaluatePredicate } from '../selection/filter';
import { spawn } from '../task';
import { CandidateChanges } from './candidate_changes';
import { GapFetcher, QueryGapFetcher } from './fetch_gap';
import { ReactorSubscriptionId } from './subscription';
import { MembershipChange, ReactorUpdate, ReactorUpdateItem } from './update';
import { WatcherChange, WatcherOp, WatcherSet } from './watcherset';
import { Predicate, Selection } from '@ankurah/ankql';
import { CollectionId, EntityId, QueryId } from '@ankurah/proto';
import { Broadcast } from '@ankurah/signals';

export class QueryState<E extends AbstractEntity & Filterable> extends Struct {
  collectionId: CollectionId;
  selection: Selection | null;
  gapFetcher: Arc<GapFetcher>;
  paused: boolean;
  resultset: EntityResultSet<E>;
  version: number;

  constructor(collectionId: CollectionId, selection: Selection | null, gapFetcher: Arc<GapFetcher>, paused: boolean, resultset: EntityResultSet<E>, version: number) {
    super();
    this.collectionId = collectionId;
    this.selection = selection;
    this.gapFetcher = gapFetcher;
    this.paused = paused;
    this.resultset = resultset;
    this.version = version;
  }
}

class Subscription<E extends AbstractEntity & Filterable, Ev extends Clone> extends Struct {
  _0: Arc<Inner<E, Ev>>;

  constructor(_0: Arc<Inner<E, Ev>>) {
    super();
    this._0 = _0;
  }

  id(): ReactorSubscriptionId {
    return this._0.value.id;
  }

  static new<E, Ev>(broadcast: Broadcast<ReactorUpdate<E, Ev>>, watcherSet: Arc<Mutex<WatcherSet>>): Subscription<E, Ev> {
    return new Subscription(Arc.new(new Inner(ReactorSubscriptionId.new(), new Mutex(new State(new HashMap(), new HashSet(), new HashMap(), broadcast)), watcherSet)));
  }

  addEntitySubscription(entityId: EntityId): void {
    let state = this.deref().state.lock();
    try {
      state.value.entitySubscriptions.add(entityId);
    } finally {
      state.drop();
    }
  }

  removeEntitySubscription(entityId: EntityId): void {
    let state = this.deref().state.lock();
    try {
      state.value.entitySubscriptions.delete(entityId);
    } finally {
      state.drop();
    }
  }

  anyQueryMatches(entityId: EntityId): boolean {
    const state = this.deref().state.lock();
    try {
      return state.value.queries.values().some((q) => q.resultset.containsKey(entityId));
    } finally {
      state.drop();
    }
  }

  systemReset(): void {
    const _t0 = this.deref().state.lock();
    try {
      const _t1 = _t0.value;
      try {
        const state = _t1;
        _t0.drop();
        try {
          let _moved2 = false;
          let updateItems = [];
          try {
            for (const [queryId, queryState] of state.queries) {
              for (const entityId of queryState.resultset.keys()) {
                {
                  const _v = state.entities.get(entityId);
                  if (_v != null) {
                    const entity = _v;
                    updateItems.push(new ReactorUpdateItem(entity.clone(), [], [[queryId, new MembershipChange('Remove', {})]]));
                  }
                }
              }
              queryState.resultset.clear();
              queryState.resultset.setLoaded(false);
            }
            state.entitySubscriptions.clear();
            state.entities.clear();
            if (!(updateItems.length === 0)) {
              _moved2 = true;
              const reactorUpdate = new ReactorUpdate(updateItems);
              state.broadcast.send(reactorUpdate);
            }
          } finally {
            if (!_moved2) dropOwned(updateItems);
          }
        } finally {
          state.drop();
        }
      } finally {
        _t1.drop();
      }
    } finally {
      _t0.drop();
    }
  }

  queriesLen(): number {
    const state = this.deref().state.lock();
    try {
      return state.value.queries.size;
    } finally {
      state.drop();
    }
  }

  registerQuery(queryId: QueryId, collectionId: CollectionId, resultset: EntityResultSet<E>, gapFetcher: Arc<GapFetcher>): Result<void, AnyhowError> {
    let _moved0 = false;
    let _moved1 = false;
    let _moved2 = false;
    try {
      try {
        try {
          let state = this.deref().state.lock();
          try {
            return state.value.queries.entry(queryId).match({
              Vacant: (_v) => {
                const v = _v._0;
                _moved0 = true;
                _moved2 = true;
                _moved1 = true;
                v.insert(new QueryState(collectionId, null, gapFetcher, false, resultset, 0));
                return Result.Ok([]);
              },
              Occupied: (v) => Result.Err(AnyhowError.msg(`Query ${queryId} already exists`)),
            });
          } finally {
            state.drop();
          }
        } finally {
          if (!_moved2) gapFetcher.drop();
        }
      } finally {
        if (!_moved1) resultset.drop();
      }
    } finally {
      if (!_moved0) collectionId.drop();
    }
  }

  updatePredicateWatchers(queryId: QueryId, collectionId: CollectionId, oldPredicate: Predicate | null, newPredicate: Predicate): void {
    let watcherSet = this.deref().watcherSet.value.lock();
    try {
      const watcherId = [this.deref().id, queryId];
      {
        const _v = oldPredicate;
        if (_v != null) {
          const oldPred = _v;
          watcherSet.value.recursePredicateWatchers(collectionId, oldPred, watcherId, new WatcherOp('Remove', {}));
        }
      }
      watcherSet.value.recursePredicateWatchers(collectionId, newPredicate, watcherId, new WatcherOp('Add', {}));
    } finally {
      watcherSet.drop();
    }
  }

  addEntityWatchers(queryId: QueryId, entityIds: EntityId[]): void {
    let watcherSet = this.deref().watcherSet.value.lock();
    try {
      watcherSet.value.addPredicateEntityWatchers(this.deref().id, queryId, entityIds);
    } finally {
      watcherSet.drop();
    }
  }

  updateQuery<A extends UpdateItemAccumulator>(queryId: QueryId, collectionId: CollectionId, selection: Selection, includedEntities: E[], version: number, reactorUpdates: A): Result<E[], Error> {
    try {
      try {
        let _moved0 = false;
        let stateGuard = this.deref().state.lock();
        try {
          const state = stateGuard.value;
          try {
            const _m1 = state.queries.get(queryId);
            const _r2 = (_m1 != null ? Result.Ok(_m1!) : Result.Err((() => AnyhowError.msg('Query not found for update'))()));
            if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
            const queryState = _r2.unwrap();
            const isFirstUpdate = (queryState.selection == null);
            const oldSelection = queryState.selection.replace(selection.clone());
            try {
              const _r3 = (selection.orderBy != null ? ((ob) => buildKeySpecFromSelection(ob.asSlice(), queryState.resultset))(selection.orderBy!) : null).transpose();
              if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
              queryState.resultset.orderBy(_r3.unwrap());
              if (isFirstUpdate || valueNotEquals((oldSelection != null ? ((s) => s.limit)(oldSelection!) : null), selection.limit)) {
                queryState.resultset.limit((selection.limit != null ? ((l) => Number(BigInt.asUintN(32, l)))(selection.limit!) : null));
              }
              let _moved4 = false;
              let rwResultset = queryState.resultset.write();
              try {
                let newlyAdded = [];
                rwResultset.markAllDirty();
                for (const entity of includedEntities) {
                  if (evaluatePredicate(entity, selection.predicate).unwrapOr(false)) {
                    const entityId = AbstractEntity.id(entity);
                    if (!rwResultset.contains(entityId)) {
                      rwResultset.add(entity.clone());
                      state.entities.set(entityId, entity.clone());
                      state.entitySubscriptions.add(entityId);
                      reactorUpdates.pushInitial(entity, queryId);
                      newlyAdded.push(entity);
                    }
                  }
                }
                let removedEntities = [];
                rwResultset.retainDirty((entity) => {
                  {
                    const _v = evaluatePredicate(entity, selection.predicate);
                    if (_v.isOk()) {
                      const _v1 = _v.unwrap();
                      return true;
                    }
                  };
                  const entityId = entity.id();
                  tracing.debug(`Entity ${entityId} no longer matches predicate`);
                  removedEntities.push(entityId);
                  reactorUpdates.pushRemove(entity, queryId);
                  return false;
                });
                queryState.paused = false;
                queryState.version = version;
                rwResultset.setLoaded(true);
                _moved4 = true;
                rwResultset.drop();
                _moved0 = true;
                stateGuard.drop();
                const shouldUpdateWatchers = (() => {
                  if (isFirstUpdate) {
                    return true;
                  } else {
                    const _v2 = oldSelection;
                    if (_v2 != null) {
                      const oldSel = _v2;
                      return !oldSel.predicate.equals(selection.predicate);
                    } else {
                    return false;
                  }
                  }
                })();
                if (shouldUpdateWatchers) {
                  const oldPred = (oldSelection != null ? ((s) => s.predicate)(oldSelection!) : null);
                  this.updatePredicateWatchers(queryId, collectionId, oldPred, selection.predicate);
                }
                if (!(newlyAdded.length === 0)) {
                  this.addEntityWatchers(queryId, [...newlyAdded].map((e) => AbstractEntity.id(e)));
                }
                if (!(removedEntities.length === 0)) {
                  let watcherSet = this.deref().watcherSet.value.lock();
                  try {
                    watcherSet.value.cleanupRemovedPredicateWatchers(this.deref().id, queryId, removedEntities);
                  } finally {
                    watcherSet.drop();
                  }
                }
                return Result.Ok(newlyAdded);
              } finally {
                if (!_moved4) rwResultset.drop();
              }
            } finally {
              dropOwned(oldSelection);
            }
          } finally {
            state.drop();
          }
        } finally {
          if (!_moved0) stateGuard.drop();
        }
      } finally {
        selection.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  sendUpdate(items: ReactorUpdateItem<E, Ev>[]): void {
    const state = this.deref().state.lock();
    try {
      state.value.broadcast.send(new ReactorUpdate(items));
    } finally {
      state.drop();
    }
  }

  removeQuery(queryId: QueryId): QueryState<E> | null {
    let state = this.deref().state.lock();
    try {
      return state.value.queries.remove(queryId);
    } finally {
      state.drop();
    }
  }

  takeAllQueries(): HashMap<QueryId, QueryState<E>> {
    let state = this.deref().state.lock();
    try {
      return mem.take(state.value.queries);
    } finally {
      state.drop();
    }
  }

  async evaluateChanges<C extends ChangeNotification & Clone>(candidates: CandidateChanges<C>): Promise<WatcherChange[]> {
    try {
      try {
        let watcherChanges = [];
        let items = IndexMap.new();
        let stateGuard = this.deref().state.lock();
        const state = stateGuard.value;
        try {
          for (const queryCandidate of candidates.queryIter()) {
            try {
              const queryId = queryCandidate.queryId;
              const _m1 = (() => {
                const _v1 = state.queries.get(queryId);
                if (_v1 != null) {
                  const qs = _v1;
                  if (!qs.paused) {
                    return qs;
                  }
                }
                {
                  return { $jump: 'continue' };
                }
              })();
              if ((_m1 as any)?.$jump === 'continue') continue;
              const queryState = (_m1 as any);
              const selection = queryState.selection.asRef();
              tracing.debug(`\tevaluate_changes query: ${queryId} ${selection}`);
              for (const change of queryCandidate.iter()) {
                const entity = change.entity();
                const entityId = AbstractEntity.id(entity);
                tracing.debug(`Subscription ${this.id()} evaluating entity ${entityId} for query ${queryId}`);
                const matches = evaluatePredicate(entity, selection.predicate).unwrapOr(false);
                const didMatch = queryState.resultset.containsKey(entityId);
                const membershipChange = (() => {
                  const _v3 = [didMatch, matches];
                  if ((_v3[0] === false) && (_v3[1] === true)) {
                    {
                      const entityClone = entity.clone();
                      queryState.resultset.write().add(entityClone.clone());
                      state.entities.set(entityId, entityClone);
                      watcherChanges.push(WatcherChange.add(entityId, this.deref().id, queryId));
                      return new MembershipChange('Add', {});
                    }
                  } else if ((_v3[0] === true) && (_v3[1] === false)) {
                    {
                      queryState.resultset.write().remove(entityId);
                      watcherChanges.push(WatcherChange.remove(entityId, this.deref().id, queryId));
                      return new MembershipChange('Remove', {});
                    }
                  } else {
                    {
                      watcherChanges.push((matches ? WatcherChange.add(entityId, this.deref().id, queryId) : WatcherChange.remove(entityId, this.deref().id, queryId)));
                      return null;
                    }
                  }
                })();
                const entitySubscribed = state.entitySubscriptions.has(entityId);
                if (matches || didMatch || entitySubscribed) {
                  const item = items.entry(entityId).orInsertWith(() => new ReactorUpdateItem(entity.clone(), change.events().map((e) => derivedClone(e)), []));
                  {
                    const _v4 = membershipChange;
                    if (_v4 != null) {
                      const mc = _v4;
                      item.predicateRelevance.push([queryId, mc]);
                    }
                  }
                }
              }
            } finally {
              queryCandidate.drop();
            }
          }
          for (const change of candidates.entityIter()) {
            const entity = change.entity();
            const entityId = AbstractEntity.id(entity);
            if (state.entitySubscriptions.has(entityId)) {
              items.entry(entityId).orInsert(new ReactorUpdateItem(entity.clone(), change.events().map((e) => derivedClone(e)), []));
            }
          }
          let _moved2 = false;
          const gapsToFill = this.collectGapsToFillInternal(state);
          try {
            let _moved3 = false;
            const broadcast = state.broadcast.clone();
            try {
              stateGuard.drop();
              let _moved4 = false;
              const updateItems = items.intoValues();
              try {
                if (!(gapsToFill.length === 0)) {
                  _moved4 = true;
                  _moved2 = true;
                  _moved3 = true;
                  spawn(this.clone().fillGapsAndNotify(updateItems, gapsToFill, broadcast));
                } else if (!(updateItems.length === 0)) {
                  _moved4 = true;
                  broadcast.send(new ReactorUpdate(updateItems));
                }
                return watcherChanges;
              } finally {
                if (!_moved4) dropOwned(updateItems);
              }
            } finally {
              if (!_moved3) broadcast.drop();
            }
          } finally {
            if (!_moved2) dropOwned(gapsToFill);
          }
        } finally {
          state.drop();
        }
      } finally {
        candidates.drop();
      }
    } finally {
      this.drop();
    }
  }

  collectGapsToFillInternal(state: State<E, Ev>): GapFillData<E>[] {
    return iterFilterMap([...state.queries], ([queryId, queryState]) => this.extractGapData(queryId, queryState));
  }

  async fillGapsForQueryEntities(queryId: QueryId, entities: E[]): Promise<void> {
    const gapData = (() => {
      const state = this.deref().state.lock();
      try {
        const _m0 = state.value.queries.get(queryId);
        return (_m0 != null ? ((queryState) => this.extractGapData(queryId, queryState))(_m0!) : null);
      } finally {
        state.drop();
      }
    })();
    const _v = gapData;
    if (!(_v != null)) {
      return;
    }
    const [queryId_1, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize] = _v;
    resultset.clearGapDirty();
    const gapFilledEntities = await Subscription.processGapFillEntities(queryId_1, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize);
    if (!(gapFilledEntities.length === 0)) {
      this.addEntityWatchers(queryId_1, [...gapFilledEntities].map((e) => AbstractEntity.id(e)));
      entities.push(...gapFilledEntities);
    }
  }

  async fillGapsForQuery<A extends UpdateItemAccumulator>(queryId: QueryId, reactorUpdates: A): Promise<void> {
    const gapData = (() => {
      const state = this.deref().state.lock();
      try {
        const _m0 = state.value.queries.get(queryId);
        return (_m0 != null ? ((queryState) => this.extractGapData(queryId, queryState))(_m0!) : null);
      } finally {
        state.drop();
      }
    })();
    const _v = gapData;
    if (!(_v != null)) {
      return;
    }
    const [queryId_1, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize] = _v;
    resultset.clearGapDirty();
    const gapFilledEntities = await Subscription.processGapFillEntities(queryId_1, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize);
    if (!(gapFilledEntities.length === 0)) {
      this.addEntityWatchers(queryId_1, [...gapFilledEntities].map((e) => AbstractEntity.id(e)));
      for (const entity of gapFilledEntities) {
        reactorUpdates.pushInitial(entity, queryId_1);
      }
    }
  }

  static async processGapFillEntities<E, Ev>(queryId: QueryId, gapFetcher: Arc<GapFetcher>, collectionId: CollectionId, selection: Selection, resultset: EntityResultSet<E>, lastEntity: E | null, gapSize: number): Promise<E[]> {
    try {
      try {
        try {
          try {
            tracing.debug(`Gap filling for query ${queryId} - need ${gapSize} entities`);
            const _v = await gapFetcher.value.fetchGap(collectionId, selection, lastEntity, gapSize);
            if (_v.isOk()) {
              const gapEntities = _v.unwrap();
              if (!(gapEntities.length === 0)) {
                tracing.debug(`Gap filling fetched ${gapEntities.length} entities for query ${queryId}`);
                let write = resultset.write();
                try {
                  let addedEntities = [];
                  for (const entity of gapEntities) {
                    if (write.add(entity.clone())) {
                      addedEntities.push(entity);
                    }
                  }
                  return addedEntities;
                } finally {
                  write.drop();
                }
              } else {
                tracing.debug(`Gap filling found no entities for query ${queryId}`);
                return [];
              }
            } else {
              const e = _v.unwrapErr();
              try {
                {
                  tracing.warn(`Gap filling failed for query ${queryId}: ${e}`);
                  return [];
                }
              } finally {
                e.drop();
              }
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
    } finally {
      gapFetcher.drop();
    }
  }

  async fillGapsAndNotify(items: ReactorUpdateItem<E, Ev>[], gapsToFill: GapFillData<E>[], broadcast: Broadcast<ReactorUpdate<E, Ev>>): Promise<void> {
    let _moved0 = false;
    try {
      try {
        try {
          for (const [, , , , resultset, , ] of gapsToFill) {
            resultset.clearGapDirty();
          }
          const gapFillFutures = [...gapsToFill].map(([queryId, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize]) => {
            return Subscription.processGapFill(queryId, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize);
          });
          const gapResults = await future.joinAll(gapFillFutures);
          const _seq2 = gapResults;
          let _at3 = 0;
          try {
            while (_at3 < _seq2.length) {
              const [queryId, gapItems] = _seq2[_at3++];
              let _moved1 = false;
              try {
                if (!(gapItems.length === 0)) {
                  const entityIds = [...gapItems].map((item) => AbstractEntity.id(item.entity));
                  this.addEntityWatchers(queryId, [...entityIds]);
                  _moved1 = true;
                  items.push(...gapItems);
                }
              } finally {
                if (!_moved1) dropOwned(gapItems);
              }
            }
          } finally {
            dropOwned(_seq2.slice(_at3));
          }
          if (!(items.length === 0)) {
            _moved0 = true;
            broadcast.send(new ReactorUpdate(items));
          }
        } finally {
          broadcast.drop();
        }
      } finally {
        if (!_moved0) dropOwned(items);
      }
    } finally {
      this.drop();
    }
  }

  extractGapData(queryId: QueryId, queryState: QueryState<E>): GapFillData<E> | null {
    const resultset = queryState.resultset;
    try {
      if (!resultset.isGapDirty()) {
        return null;
      }
      const _r0 = resultset.getLimit();
      if (_r0 == null) return null;
      const limit = _r0;
      const currentLen = resultset.len();
      if (currentLen >= limit) {
        return null;
      }
      const gapSize = checkedSub(limit, currentLen, 'usize');
      const lastEntity = resultset.lastEntity();
      let _moved1 = false;
      const selection = (queryState.selection.clone() ?? (() => { throw new Error('extract_gap_data called before update_query'); })());
      try {
        _moved1 = true;
        return [queryId, queryState.gapFetcher.clone(), queryState.collectionId.clone(), selection, resultset.clone(), lastEntity, gapSize];
      } finally {
        if (!_moved1) selection.drop();
      }
    } finally {
      resultset.drop();
    }
  }

  static async processGapFill<E, Ev>(queryId: QueryId, gapFetcher: Arc<GapFetcher>, collectionId: CollectionId, selection: Selection, resultset: EntityResultSet<E>, lastEntity: E | null, gapSize: number): Promise<[QueryId, ReactorUpdateItem<E, Ev>[]]> {
    try {
      try {
        try {
          try {
            tracing.debug(`Gap filling for query ${queryId} - need ${gapSize} entities`);
            const gapItems = await (async () => {
              const _v1 = await gapFetcher.value.fetchGap(collectionId, selection, lastEntity, gapSize);
              if (_v1.isOk()) {
                const gapEntities = _v1.unwrap();
                if (!(gapEntities.length === 0)) {
                  tracing.debug(`Gap filling fetched ${gapEntities.length} entities for query ${queryId}`);
                  let write = resultset.write();
                  try {
                    let gapItems = [];
                    for (const entity of gapEntities) {
                      if (write.add(entity.clone())) {
                        gapItems.push(new ReactorUpdateItem(entity, [], [[queryId, new MembershipChange('Add', {})]]));
                      }
                    }
                    return gapItems;
                  } finally {
                    write.drop();
                  }
                } else {
                  tracing.debug(`Gap filling found no entities for query ${queryId}`);
                  return [];
                }
              } else {
                const e = _v1.unwrapErr();
                try {
                  {
                    tracing.warn(`Gap filling failed for query ${queryId}: ${e}`);
                    return [];
                  }
                } finally {
                  e.drop();
                }
              }
            })();
            return [queryId, gapItems];
          } finally {
            resultset.drop();
          }
        } finally {
          selection.drop();
        }
      } finally {
        collectionId.drop();
      }
    } finally {
      gapFetcher.drop();
    }
  }

  upsertQuery<SE, PA>(queryId: QueryId, collectionId: CollectionId, node: Node<SE, PA>, cdata: ContextData): EntityResultSet<Entity> {
    let _moved0 = false;
    try {
      let state = this.deref().state.lock();
      try {
        return state.value.queries.entry(queryId).match({
          Vacant: (_v) => {
            const v = _v._0;
            const resultset = EntityResultSet.empty();
            const gapFetcher = Arc.new(QueryGapFetcher.new(node, cdata.clone()));
            _moved0 = true;
            v.insert(new QueryState(collectionId, null, gapFetcher, false, resultset.clone(), 0));
            return resultset;
          },
          Occupied: (v) => {
            const o = v._0;
            return o.get().resultset.clone();
          },
        });
      } finally {
        state.drop();
      }
    } finally {
      if (!_moved0) collectionId.drop();
    }
  }

  clone(): Subscription<E, Ev> {
    return new Subscription(this._0.clone());
  }

  deref(): Inner<E, Ev> {
    return this._0;
  }
}

class Inner<E extends AbstractEntity & Filterable, Ev> extends Struct {
  id: ReactorSubscriptionId;
  state: Mutex<State<E, Ev>>;
  watcherSet: Arc<Mutex<WatcherSet>>;

  constructor(id: ReactorSubscriptionId, state: Mutex<State<E, Ev>>, watcherSet: Arc<Mutex<WatcherSet>>) {
    super();
    this.id = id;
    this.state = state;
    this.watcherSet = watcherSet;
  }
}

class State<E extends AbstractEntity & Filterable, Ev> extends Struct {
  queries: HashMap<QueryId, QueryState<E>>;
  entitySubscriptions: HashSet<EntityId>;
  entities: HashMap<EntityId, E>;
  broadcast: Broadcast<ReactorUpdate<E, Ev>>;

  constructor(queries: HashMap<QueryId, QueryState<E>>, entitySubscriptions: HashSet<EntityId>, entities: HashMap<EntityId, E>, broadcast: Broadcast<ReactorUpdate<E, Ev>>) {
    super();
    this.queries = queries;
    this.entitySubscriptions = entitySubscriptions;
    this.entities = entities;
    this.broadcast = broadcast;
  }
}

export interface UpdateItemAccumulator<E, Ev> {
  pushInitial(entity: E, queryId: QueryId): void;
  pushRemove(entity: E, queryId: QueryId): void;
}

type GapFillData = [QueryId, Arc<GapFetcher>, CollectionId, Selection, EntityResultSet<E>, E | null, number];

export function Vec_ReactorUpdateItem_pushInitial<E extends Clone, Ev>(self: ReactorUpdateItem<E, Ev>[], entity: E, queryId: QueryId): void {
  Vec.push(self, new ReactorUpdateItem(entity.clone(), [], [[queryId, new MembershipChange('Initial', {})]]));
}

export function Vec_ReactorUpdateItem_pushRemove<E extends Clone, Ev>(self: ReactorUpdateItem<E, Ev>[], entity: E, queryId: QueryId): void {
  Vec.push(self, new ReactorUpdateItem(entity.clone(), [], [[queryId, new MembershipChange('Remove', {})]]));
}

export function Unit_pushInitial<E, Ev>(self: void, _entity: E, _queryId: QueryId): void {

}

export function Unit_pushRemove<E, Ev>(self: void, _entity: E, _queryId: QueryId): void {

}

