// MIRRORS: ankurah/storage/indexeddb-wasm/src/collection.rs
import { Struct, Result, Arc, dropOwned, valueEquals, tracing, unsupported, checkedAdd, wrappingAdd, iterFirst, HashMap, HashSet, AsyncMutex } from '@ankurah/base';
import { ComparisonOperator, Expr, Literal, PathExpr, Predicate, Selection } from '@ankurah/ankql';
import { Filterable, MutationError, RetrievalError, StorageCollection, State, Value, backendFromString, evaluatePredicate } from '@ankurah/core';
import { Attested, EntityId, EntityState, EventId, State, CollectionId, Event } from '@ankurah/proto';
import { OrderByComponents, HasEntityId, Planner, PlannerConfig } from '@ankurah/storage-common';
import { Database } from './database';
import { IdbValue } from './idb_value';
import { planBoundsToIdbRange, scanDirectionToCursorDirection } from './planner_integration';
import { IdbIndexScanner } from './scanner';
import { cbFuture } from './util/cb_future';
import { cbStream } from './util/cb_stream';
import { Object } from './util/object';
import { Result_JsValue_require } from './util/require';

export class IndexedDBBucket extends Struct implements StorageCollection {
  db: Database;
  collectionId: CollectionId;
  mutex: AsyncMutex<void>;
  invocationCount: number;
  prefixGuardDisabled: Arc<boolean>;

  constructor(db: Database, collectionId: CollectionId, mutex: AsyncMutex<void>, invocationCount: number, prefixGuardDisabled: Arc<boolean>) {
    super();
    this.db = db;
    this.collectionId = collectionId;
    this.mutex = mutex;
    this.invocationCount = invocationCount;
    this.prefixGuardDisabled = prefixGuardDisabled;
  }

  async executePlanQuery(index: IdbIndex, keyRange: IdbKeyRange | null, predicate: Predicate, cursorDirection: IdbCursorDirection, limit: bigint | null, collectionId: CollectionId, upperOpenEnded: boolean, eqPrefixLen: number, eqPrefixValues: Value[], orderBySpill: OrderByComponents): Promise<Result<Attested<EntityState>[], RetrievalError>> {
    const needsSpillSort = !(orderBySpill.spill.length === 0);
    const effectivePrefixLen = (upperOpenEnded && eqPrefixLen > 0 && !this.prefixGuardDisabled.value ? eqPrefixLen : 0);
    const scanner = IdbIndexScanner.new(index.clone(), keyRange, cursorDirection, effectivePrefixLen, eqPrefixValues);
    try {
      let stream = undefined /* pin!(scanner . scan ()) */;
      let count = 0n;
      let rows = [];
      try {
        let _moved1 = false;
        let directResults = [];
        try {
          for (;;) {
            const _v6 = await stream.next();
            if (!(_v6 != null)) {
              break;
            }
            const result = _v6;
            const _r2 = result;
            if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
            const entityObj = _r2.unwrap();
            const _m3 = (() => {
              const _v2 = IdbRecord.new(entityObj, collectionId.clone());
              if (_v2.isOk()) {
                const r = _v2.unwrap();
                return r;
              } else {
                const _v3 = _v2.unwrapErr();
                try {
                  return { $jump: 'continue' };
                } finally {
                  _v3.drop();
                }
              }
            })();
            if ((_m3 as any)?.$jump === 'continue') continue;
            const record = (_m3 as any);
            let _c5;
            const _r4 = evaluatePredicate(record, predicate).mapErr((e) => {
              try {
                return new RetrievalError('StorageError', { _0: `Predicate evaluation failed: ${e}` });
              } finally {
                e.drop();
              }
            });
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            _c5 = _r4.unwrap();
            if (_c5) {
              if (needsSpillSort) {
                rows.push(record);
              } else {
                {
                  const _v5 = record.entityState();
                  if (_v5.isOk()) {
                    const entityState = _v5.unwrap();
                    directResults.push(entityState);
                    count = checkedAdd(count, 1n, 'u64');
                    {
                      const _v4 = limit;
                      if (_v4 != null) {
                        const limitVal = _v4;
                        if (count >= limitVal) {
                          break;
                        }
                      }
                    }
                  }
                }
              }
            }
          }
          if (needsSpillSort) {
            const results = await (async () => {
              if (limit != null) {
                const limitVal = limit;
                let _moved11 = false;
                const _b10 = orderBySpill.clone();
                try {
                  const _b12 = Number(BigInt.asUintN(32, limitVal));
                  _moved11 = true;
                  return await unsupported('`collect` into `Collect<FilterMap<TopKStream<Iter<IntoIter>>, Fut, F>, C>` is a `FromIterator` the port has no construction for');
                } finally {
                  if (!_moved11) dropOwned(_b10);
                }
              } else {
                const _b13 = orderBySpill.clone();
                return await unsupported('`collect` into `Collect<FilterMap<SortedStream<Iter<IntoIter>>, Fut, F>, C>` is a `FromIterator` the port has no construction for');
              }
            })();
            return Result.Ok(results);
          } else {
            _moved1 = true;
            return Result.Ok(directResults);
          }
        } finally {
          if (!_moved1) dropOwned(directResults);
        }
      } finally {
        dropOwned(rows);
      }
    } finally {
      scanner.drop();
    }
  }

  toString(): string {
    return `IndexedDBBucket(${this.collectionId})`;
  }

  async setState(state: Attested<EntityState>): Promise<Result<boolean, MutationError>> {
    try {
      (() => { const _v = this.invocationCount; this.invocationCount = wrappingAdd(this.invocationCount, 1, 'usize'); return _v; })();
      const _lock = await this.mutex.lock();
      try {
        const dbConnection = await this.db.getConnection();
        return await SendWrapper.new((async () => {
          undefined /* action_debug!(self , "set_state {}" , "{}" , & self . collection_id) */;
          const _r0 = Result_JsValue_require(dbConnection.transactionWithStrAndMode('entities', Readwrite), 'create transaction');
          if (_r0.isErr()) return Result.Err(MutationError.fromAnyhowError(_r0.unwrapErr()));
          const transaction = _r0.unwrap();
          const _r1 = Result_JsValue_require(transaction.objectStore('entities'), 'get object store');
          if (_r1.isErr()) return Result.Err(MutationError.fromAnyhowError(_r1.unwrapErr()));
          const store = _r1.unwrap();
          const _r2 = Result_JsValue_require(store.get(state.payload.entityId.toString()), 'get old entity');
          if (_r2.isErr()) return Result.Err(MutationError.fromAnyhowError(_r2.unwrapErr()));
          const oldRequest = _r2.unwrap();
          const foo = await cbFuture(oldRequest, 'success', 'error');
          try {
            const _r3 = foo.require('get old entity');
            if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
            const _ = _r3.unwrap();
            const _r4 = Result_JsValue_require(oldRequest.result(), 'get old entity result');
            if (_r4.isErr()) return Result.Err(MutationError.fromAnyhowError(_r4.unwrapErr()));
            const oldEntity = _r4.unwrap();
            if (!((oldEntity === undefined)) && !((oldEntity === null))) {
              const oldEntityObj = Object.new(oldEntity);
              try {
                const _r5 = oldEntityObj.getOpt(HEAD_KEY);
                if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
                {
                  const _v = _r5.unwrap();
                  if (_v != null) {
                    const oldClock = _v;
                    if (valueEquals(oldClock, state.payload.state.head)) {
                      return Result.Ok(false);
                    }
                  }
                }
              } finally {
                oldEntityObj.drop();
              }
            }
            const entity = Object.new(jsSys.Object.new());
            try {
              const _r6 = entity.set(ID_KEY, state.payload.entityId.toString());
              if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
              _r6.drop();
              const _r7 = entity.set(COLLECTION_KEY, this.collectionId.asStr());
              if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
              _r7.drop();
              const _r8 = entity.set(STATE_BUFFER_KEY, state.payload.state.stateBuffers);
              if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
              _r8.drop();
              const _r9 = entity.set(HEAD_KEY, state.payload.state.head);
              if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
              _r9.drop();
              const _r10 = entity.set(ATTESTATIONS_KEY, state.attestations);
              if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
              _r10.drop();
              const _r11 = extractAllFields(entity, state.payload);
              if (_r11.isErr()) return Result.Err(_r11.unwrapErr());
              _r11.drop();
              const _r12 = Result_JsValue_require(store.putWithKey(entity, state.payload.entityId.toString()), 'put entity in store');
              if (_r12.isErr()) return Result.Err(MutationError.fromAnyhowError(_r12.unwrapErr()));
              const request = _r12.unwrap();
              const _t13 = await cbFuture(request, 'success', 'error');
              try {
                const _r14 = _t13.require('put entity in store');
                if (_r14.isErr()) return Result.Err(_r14.unwrapErr());
                _r14.drop();
              } finally {
                _t13.drop();
              }
              const _t15 = await cbFuture(transaction, 'complete', 'error');
              try {
                const _r16 = _t15.require('complete transaction');
                if (_r16.isErr()) return Result.Err(_r16.unwrapErr());
                _r16.drop();
              } finally {
                _t15.drop();
              }
              return Result.Ok(true);
            } finally {
              entity.drop();
            }
          } finally {
            foo.drop();
          }
        })());
      } finally {
        _lock.drop();
      }
    } finally {
      state.drop();
    }
  }

  async getState(id: EntityId): Promise<Result<Attested<EntityState>, RetrievalError>> {
    const dbConnection = await this.db.getConnection();
    return await SendWrapper.new((async () => {
      const _r0 = Result_JsValue_require(dbConnection.transactionWithStr('entities'), 'create transaction');
      if (_r0.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r0.unwrapErr()));
      const transaction = _r0.unwrap();
      const _r1 = Result_JsValue_require(transaction.objectStore('entities'), 'get object store');
      if (_r1.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r1.unwrapErr()));
      const store = _r1.unwrap();
      const _r2 = Result_JsValue_require(store.get(id.toString()), 'get entity');
      if (_r2.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r2.unwrapErr()));
      const request = _r2.unwrap();
      const _t3 = await cbFuture(request, 'success', 'error');
      try {
        const _r4 = _t3.require('await request');
        if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
        _r4.drop();
      } finally {
        _t3.drop();
      }
      const _r5 = Result_JsValue_require(request.result(), 'get result');
      if (_r5.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r5.unwrapErr()));
      const result = _r5.unwrap();
      if ((result === undefined) || (result === null)) {
        return Result.Err(new RetrievalError('EntityNotFound', { _0: id }));
      }
      const entity = Object.new(result);
      try {
        const _r6 = entity.get(STATE_BUFFER_KEY);
        if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
        try {
          const _r7 = entity.get(HEAD_KEY);
          if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
          try {
            const _r8 = entity.get(ATTESTATIONS_KEY);
            if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
            try {
              return Result.Ok(new Attested(new EntityState(id, this.collectionId.clone(), new State(_r6.unwrap(), _r7.unwrap())), _r8.unwrap()));
            } finally {
              if (_r8 != null && !(_r8 as any).isMoved && !(_r8 as any).isDropped) dropOwned(_r8);
            }
          } finally {
            if (_r7 != null && !(_r7 as any).isMoved && !(_r7 as any).isDropped) dropOwned(_r7);
          }
        } finally {
          if (_r6 != null && !(_r6 as any).isMoved && !(_r6 as any).isDropped) dropOwned(_r6);
        }
      } finally {
        entity.drop();
      }
    })());
  }

  async fetchStates(selection: Selection): Promise<Result<Attested<EntityState>[], RetrievalError>> {
    const _invocation = (() => { const _v = this.invocationCount; this.invocationCount = wrappingAdd(this.invocationCount, 1, 'usize'); return _v; })();
    const _lock = await this.mutex.lock();
    try {
      const amendedSelection = addCollection(selection, this.collectionId);
      try {
        const planner = Planner.new(PlannerConfig.indexeddb());
        try {
          const plans = planner.plan(amendedSelection, 'id');
          try {
            const _m0 = iterFirst(plans);
            const _r1 = (_m0 != null ? Result.Ok(_m0!) : Result.Err((() => new RetrievalError('StorageError', { _0: 'No plan generated' }))()));
            if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
            const plan = _r1.unwrap();
            const _m9 = await (async () => {
              return await (plan.match<any>({
                EmptyScan: async () => {
                  return { $jump: 'return', $value: Result.Ok([]) };
                },
                Index: async (v) => {
                  const indexSpec = v.indexSpec;
                  const bounds = v.bounds;
                  const scanDirection = v.scanDirection;
                  const remainingPredicate = v.remainingPredicate;
                  const orderBySpill = v.orderBySpill;
                  const _r2 = (await this.db.assureIndexExists(indexSpec)).mapErr((e) => {
                    try {
                      return new RetrievalError('StorageError', { _0: `ensure index exists: ${e}` });
                    } finally {
                      e.drop();
                    }
                  });
                  if (_r2.isErr()) return { $jump: 'return', $value: Result.Err(_r2.unwrapErr()) };
                  _r2.drop();
                  const dbConnection = await this.db.getConnection();
                  const collectionId = this.collectionId.clone();
                  try {
                    const limit = selection.limit;
                    return SendWrapper.new((async () => {
                      const _r3 = Result_JsValue_require(dbConnection.transactionWithStr('entities'), 'create transaction');
                      if (_r3.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r3.unwrapErr()));
                      const transaction = _r3.unwrap();
                      const _r4 = Result_JsValue_require(transaction.objectStore('entities'), 'get object store');
                      if (_r4.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r4.unwrapErr()));
                      const store = _r4.unwrap();
                      const _r5 = Result_JsValue_require(store.index(indexSpec.nameWith('', '__')), 'get index');
                      if (_r5.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r5.unwrapErr()));
                      const index = _r5.unwrap();
                      const _r6 = planBoundsToIdbRange(bounds, scanDirection).mapErr((e) => new RetrievalError('StorageError', { _0: `bounds conversion: ${e}` }));
                      if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
                      const [keyRange, upperOpenEnded, eqPrefixLen, eqPrefixValues] = _r6.unwrap();
                      const cursorDirection = scanDirectionToCursorDirection(scanDirection);
                      const _r7 = await this.executePlanQuery(index, keyRange, remainingPredicate, cursorDirection, limit, collectionId, upperOpenEnded, eqPrefixLen, eqPrefixValues, orderBySpill);
                      if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
                      let _moved8 = false;
                      const results = _r7.unwrap();
                      try {
                        _moved8 = true;
                        return Result.Ok(results);
                      } finally {
                        if (!_moved8) dropOwned(results);
                      }
                    })());
                  } finally {
                    collectionId.drop();
                  }
                },
                TableScan: async () => {
                  throw new Error('We should always have an IndexPlan or EmptyScan due to the amendment of the selection to include the collection');
                },
              }));
            })();
            if ((_m9 as any)?.$jump === 'return') return (_m9 as any).$value;
            return await (_m9 as any);
          } finally {
            dropOwned(plans);
          }
        } finally {
          planner.drop();
        }
      } finally {
        amendedSelection.drop();
      }
    } finally {
      _lock.drop();
    }
  }

  async addEvent(attestedEvent: Attested<Event>): Promise<Result<boolean, MutationError>> {
    const invocation = (() => { const _v = this.invocationCount; this.invocationCount = wrappingAdd(this.invocationCount, 1, 'usize'); return _v; })();
    tracing.debug(`IndexedDBBucket(${this.collectionId}).add_event(${invocation})`);
    const _lock = await this.mutex.lock();
    try {
      tracing.debug(`IndexedDBBucket(${this.collectionId}).add_event(${invocation}) LOCKED`);
      const dbConnection = await this.db.getConnection();
      return await SendWrapper.new((async () => {
        const _r0 = Result_JsValue_require(dbConnection.transactionWithStrAndMode('events', webSys.IdbTransactionMode.Readwrite), 'create transaction');
        if (_r0.isErr()) return Result.Err(MutationError.fromAnyhowError(_r0.unwrapErr()));
        const transaction = _r0.unwrap();
        const _r1 = Result_JsValue_require(transaction.objectStore('events'), 'get object store');
        if (_r1.isErr()) return Result.Err(MutationError.fromAnyhowError(_r1.unwrapErr()));
        const store = _r1.unwrap();
        const eventObj = Object.new(jsSys.Object.new());
        try {
          const payload = attestedEvent.payload;
          try {
            const _t2 = payload.id();
            try {
              const _r3 = eventObj.set(ID_KEY, _t2);
              if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
              _r3.drop();
            } finally {
              _t2.drop();
            }
            const _r4 = eventObj.set(ENTITY_ID_KEY, payload.entityId.toBase64());
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            _r4.drop();
            const _r5 = eventObj.set(OPERATIONS_KEY, payload.operations);
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            _r5.drop();
            const _r6 = eventObj.set(ATTESTATIONS_KEY, attestedEvent.attestations);
            if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
            _r6.drop();
            const _r7 = eventObj.set(PARENT_KEY, payload.parent);
            if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
            _r7.drop();
            const _t8 = payload.id();
            try {
              const _r9 = Result_JsValue_require(store.putWithKey(eventObj, (_t8)), 'put event in store');
              if (_r9.isErr()) return Result.Err(MutationError.fromAnyhowError(_r9.unwrapErr()));
              const request = _r9.unwrap();
              const _t10 = await cbFuture(request, 'success', 'error');
              try {
                const _r11 = _t10.require('await request');
                if (_r11.isErr()) return Result.Err(_r11.unwrapErr());
                _r11.drop();
              } finally {
                _t10.drop();
              }
              const _t12 = await cbFuture(transaction, 'complete', 'error');
              try {
                const _r13 = _t12.require('complete transaction');
                if (_r13.isErr()) return Result.Err(_r13.unwrapErr());
                _r13.drop();
              } finally {
                _t12.drop();
              }
              return Result.Ok(true);
            } finally {
              _t8.drop();
            }
          } finally {
            payload.drop();
          }
        } finally {
          eventObj.drop();
        }
      })());
    } finally {
      _lock.drop();
    }
  }

  async getEvents(eventIds: EventId[]): Promise<Result<Attested<Event>[], RetrievalError>> {
    let _moved0 = false;
    try {
      if (eventIds.length === 0) {
        return Result.Ok([]);
      }
      const dbConnection = await this.db.getConnection();
      return await SendWrapper.new((async () => {
        const _r1 = Result_JsValue_require(dbConnection.transactionWithStr('events'), 'create transaction');
        if (_r1.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r1.unwrapErr()));
        const transaction = _r1.unwrap();
        const _r2 = Result_JsValue_require(transaction.objectStore('events'), 'get object store');
        if (_r2.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r2.unwrapErr()));
        const store = _r2.unwrap();
        let events = [];
        _moved0 = true;
        const _seq11 = eventIds;
        let _at12 = 0;
        try {
          while (_at12 < _seq11.length) {
            const eventId = _seq11[_at12++];
            try {
              const _r3 = Result_JsValue_require(store.get(eventId.toBase64()), 'get event');
              if (_r3.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r3.unwrapErr()));
              const request = _r3.unwrap();
              const _t4 = await cbFuture(request, 'success', 'error');
              try {
                const _r5 = _t4.require('await event request');
                if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
                _r5.drop();
              } finally {
                _t4.drop();
              }
              const _r6 = Result_JsValue_require(request.result(), 'get result');
              if (_r6.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r6.unwrapErr()));
              const result = _r6.unwrap();
              if ((result === undefined) || (result === null)) {
                continue;
              }
              const eventObj = Object.new(result);
              try {
                const _r7 = eventObj.get(ENTITY_ID_KEY);
                if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
                try {
                  const _r8 = eventObj.get(OPERATIONS_KEY);
                  if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
                  try {
                    const _r9 = eventObj.get(PARENT_KEY);
                    if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
                    try {
                      const _r10 = eventObj.get(ATTESTATIONS_KEY);
                      if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
                      try {
                        const event = new Attested(new Event(this.collectionId.clone(), _r7.unwrap(), _r8.unwrap(), _r9.unwrap()), _r10.unwrap());
                        events.push(event);
                      } finally {
                        if (_r10 != null && !(_r10 as any).isMoved && !(_r10 as any).isDropped) dropOwned(_r10);
                      }
                    } finally {
                      if (_r9 != null && !(_r9 as any).isMoved && !(_r9 as any).isDropped) dropOwned(_r9);
                    }
                  } finally {
                    if (_r8 != null && !(_r8 as any).isMoved && !(_r8 as any).isDropped) dropOwned(_r8);
                  }
                } finally {
                  if (_r7 != null && !(_r7 as any).isMoved && !(_r7 as any).isDropped) dropOwned(_r7);
                }
              } finally {
                eventObj.drop();
              }
            } finally {
              eventId.drop();
            }
          }
        } finally {
          dropOwned(_seq11.slice(_at12));
        }
        return Result.Ok(events);
      })());
    } finally {
      if (!_moved0) dropOwned(eventIds);
    }
  }

  async dumpEntityEvents(id: EntityId): Promise<Result<Attested<Event>[], RetrievalError>> {
    const dbConnection = await this.db.getConnection();
    return await SendWrapper.new((async () => {
      const _r0 = Result_JsValue_require(dbConnection.transactionWithStr('events'), 'create transaction');
      if (_r0.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r0.unwrapErr()));
      const transaction = _r0.unwrap();
      const _r1 = Result_JsValue_require(transaction.objectStore('events'), 'get object store');
      if (_r1.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r1.unwrapErr()));
      const store = _r1.unwrap();
      const _r2 = Result_JsValue_require(store.index('by_entity_id'), 'get entity_id index');
      if (_r2.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r2.unwrapErr()));
      const index = _r2.unwrap();
      const _r3 = Result_JsValue_require(webSys.IdbKeyRange.only(id), 'create key range');
      if (_r3.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r3.unwrapErr()));
      const keyRange = _r3.unwrap();
      const _r4 = Result_JsValue_require(index.openCursorWithRange(keyRange), 'open cursor');
      if (_r4.isErr()) return Result.Err(RetrievalError.fromAnyhowError(_r4.unwrapErr()));
      const request = _r4.unwrap();
      let events = [];
      let stream = cbStream(request, 'success', 'error');
      try {
        for (;;) {
          const _v = await stream.next();
          if (!(_v != null)) {
            break;
          }
          const result = _v;
          const _r5 = result.require('Cursor error');
          if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
          const cursorResult = _r5.unwrap();
          if (cursorResult.isNull() || cursorResult.isUndefined()) {
            break;
          }
          const _r6 = cursorResult.dynInto().require('cast cursor');
          if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
          const cursor = _r6.unwrap();
          const _r7 = cursor.value().require('get cursor value');
          if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
          const eventObj = Object.new(_r7.unwrap());
          try {
            const _r8 = eventObj.get(ENTITY_ID_KEY);
            if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
            try {
              const _r9 = eventObj.get(OPERATIONS_KEY);
              if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
              try {
                const _r10 = eventObj.get(PARENT_KEY);
                if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
                try {
                  const _r11 = eventObj.get(ATTESTATIONS_KEY);
                  if (_r11.isErr()) return Result.Err(_r11.unwrapErr());
                  try {
                    const event = new Attested(new Event(this.collectionId.clone(), _r8.unwrap(), _r9.unwrap(), _r10.unwrap()), _r11.unwrap());
                    events.push(event);
                    const _r12 = cursor.continue().require('Failed to advance cursor');
                    if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
                    _r12.drop();
                  } finally {
                    if (_r11 != null && !(_r11 as any).isMoved && !(_r11 as any).isDropped) dropOwned(_r11);
                  }
                } finally {
                  if (_r10 != null && !(_r10 as any).isMoved && !(_r10 as any).isDropped) dropOwned(_r10);
                }
              } finally {
                if (_r9 != null && !(_r9 as any).isMoved && !(_r9 as any).isDropped) dropOwned(_r9);
              }
            } finally {
              if (_r8 != null && !(_r8 as any).isMoved && !(_r8 as any).isDropped) dropOwned(_r8);
            }
          } finally {
            eventObj.drop();
          }
        }
        return Result.Ok(events);
      } finally {
        stream.drop();
      }
    })());
  }

  debug(): string {
    return `IndexedDBBucket { db: ${this.db}, collectionId: ${this.collectionId.debug()}, mutex: ${this.mutex}, invocationCount: ${this.invocationCount}, prefixGuardDisabled: ${this.prefixGuardDisabled} }`;
  }
}

class IdbRecord extends Struct implements Filterable, HasEntityId {
  id: EntityId;
  object: Object;
  collectionId: CollectionId;

  constructor(id: EntityId, object: Object, collectionId: CollectionId) {
    super();
    this.id = id;
    this.object = object;
    this.collectionId = collectionId;
  }

  static new(object: Object, collectionId: CollectionId): Result<IdbRecord, RetrievalError> {
    let _moved0 = false;
    let _moved1 = false;
    try {
      try {
        const _r2 = object.get(ID_KEY);
        if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
        const id = _r2.unwrap();
        _moved0 = true;
        _moved1 = true;
        return Result.Ok(new IdbRecord(id, object, collectionId));
      } finally {
        if (!_moved1) collectionId.drop();
      }
    } finally {
      if (!_moved0) object.drop();
    }
  }

  entityState(): Result<Attested<EntityState>, RetrievalError> {
    return jsObjectToEntityState(this.object, this.collectionId);
  }

  extractSortProperties(orderBy: OrderByComponents): HashMap<string, Value> {
    return extractSortProperties(this.object, orderBy);
  }

  collection(): string {
    return this.collectionId.asStr();
  }

  value(name: string): Value | null {
    const _r0 = this.object.getOpt(name).ok();
    if (_r0 == null) return null;
    const _r1 = _r0;
    if (_r1 == null) return null;
    let _moved2 = false;
    const idbVal = _r1;
    try {
      _moved2 = true;
      return idbVal.intoValue();
    } finally {
      if (!_moved2) idbVal.drop();
    }
  }

  entityId(): EntityId {
    return this.id;
  }
}

function extractSortProperties(entityObj: Object, orderBy: OrderByComponents): HashMap<string, Value> {
  let map = new HashMap();
  for (const item of orderBy.presort) {
    const propertyName = item.path.property();
    {
      const _v = entityObj.getOpt(propertyName);
      if (_v.isOk()) {
        const _v1 = _v.unwrap();
        map.insert(propertyName, idbVal.intoValue());
      } else {
      _v.drop();
    }
    }
  }
  for (const item of orderBy.spill) {
    const propertyName = item.path.property();
    {
      const _v2 = entityObj.getOpt(propertyName);
      if (_v2.isOk()) {
        const _v3 = _v2.unwrap();
        map.insert(propertyName, idbVal.intoValue());
      } else {
      _v2.drop();
    }
    }
  }
  return map;
}

function jsObjectToEntityState(entityObj: Object, collectionId: CollectionId): Result<Attested<EntityState>, RetrievalError> {
  const _r0 = entityObj.get(ID_KEY);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const id = _r0.unwrap();
  const _r1 = entityObj.get(STATE_BUFFER_KEY);
  if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
  try {
    const _r2 = entityObj.get(HEAD_KEY);
    if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
    try {
      let _moved3 = false;
      const entityState = new EntityState(id, collectionId.clone(), new State(_r1.unwrap(), _r2.unwrap()));
      try {
        const _r4 = entityObj.get(ATTESTATIONS_KEY);
        if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
        const attestations = _r4.unwrap();
        _moved3 = true;
        const attestedState = new Attested(entityState, attestations);
        return Result.Ok(attestedState);
      } finally {
        if (!_moved3) entityState.drop();
      }
    } finally {
      if (_r2 != null && !(_r2 as any).isMoved && !(_r2 as any).isDropped) dropOwned(_r2);
    }
  } finally {
    if (_r1 != null && !(_r1 as any).isMoved && !(_r1 as any).isDropped) dropOwned(_r1);
  }
}

function extractAllFields(entityObj: Object, entityState: EntityState): Result<void, MutationError> {
  let seenFields = new HashSet();
  for (const [backendName, stateBuffer] of [...entityState.state.stateBuffers.deref()]) {
    const _r0 = backendFromString(backendName, stateBuffer).mapErr((e) => new MutationError('General', { _0: e }));
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const backend = _r0.unwrap();
    try {
      const _seq3 = backend.value.propertyValues().intoEntries();
      let _at4 = 0;
      try {
        while (_at4 < _seq3.length) {
          const [fieldName, value] = _seq3[_at4++];
          try {
            if (!seenFields.insert(fieldName)) {
              continue;
            }
            const jsValue = (() => {
              if (value != null) {
                const propValue = value;
                return IdbValue.fromRefValue(propValue);
              } else {
                return JsValue.NULL;
              }
            })();
            const _r2 = entityObj.set(fieldName, jsValue);
            if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
            _r2.drop();
          } finally {
            dropOwned(value);
          }
        }
      } finally {
        dropOwned(_seq3.slice(_at4));
      }
    } finally {
      backend.drop();
    }
  }
  return Result.Ok([]);
}

export function addCollection(selection: Selection, collectionId: CollectionId): Selection {
  const collectionComparison = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('__collection') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: collectionId.toString() }) }) });
  return new Selection(new Predicate('And', { _0: collectionComparison, _1: selection.predicate.clone() }), selection.orderBy.clone(), selection.limit);
}

