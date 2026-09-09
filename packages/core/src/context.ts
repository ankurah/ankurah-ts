// MIRRORS: ankurah/core/src/context.rs
import { Struct, Result, Arc, dropOwned, OwnershipFatal, UnsupportedShape, tracing, dropUnbound, unsupported, iterFirst, debugString } from '@ankurah/base';
import { Attested, Clock, CollectionId, EntityState, EntityId, Event, NodeRequestBody } from '@ankurah/proto';
import { EntityChange } from './changes';
import { Entity } from './entity';
import { MutationError, RetrievalError } from './error';
import { View } from './indexel';
import { EntityLiveQuery, LiveQuery } from './livequery';
import { ContextData, MatchArgs, Node } from './node';
import { NodeApplier } from './node_applier';
import { AccessDenied, PolicyAgent } from './policy';
import { EphemeralNodeRetriever } from './retrieval';
import { StorageCollectionWrapper, StorageEngine } from './storage';
import { Transaction } from './transaction';
import { Selection } from '@ankurah/ankql';

export class Context extends Struct {
  _0: Arc<TContext>;

  constructor(_0: Arc<TContext>) {
    super();
    this._0 = _0;
  }

  begin(): Transaction {
    return Transaction.new(this._0.clone());
  }

  static new<SE extends StorageEngine, PA extends PolicyAgent>(node: Node<SE, PA>, data: ContextData): Context {
    return new Context(Arc.new(new NodeAndContext(node, data)));
  }

  nodeId(): EntityId {
    return this._0.value.nodeId();
  }

  async get<R extends View>(id: EntityId): Promise<Result<R, RetrievalError>> {
    const _r0 = await this._0.value.getEntity(id, R.collection(), false);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const entity = _r0.unwrap();
    try {
      _moved1 = true;
      return Result.Ok(R.fromEntity(entity));
    } finally {
      if (!_moved1) entity.drop();
    }
  }

  async getCached<R extends View>(id: EntityId): Promise<Result<R, RetrievalError>> {
    const _r0 = await this._0.value.getEntity(id, R.collection(), true);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const entity = _r0.unwrap();
    try {
      _moved1 = true;
      return Result.Ok(R.fromEntity(entity));
    } finally {
      if (!_moved1) entity.drop();
    }
  }

  async fetch<R extends View>(args: TryInto): Promise<Result<R[], RetrievalError>> {
    const _r0 = args.tryInto().mapErr((e) => e);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const args_1 = _r0.unwrap();
    try {
      const collectionId = R.Model.collection();
      _moved1 = true;
      const _r2 = await this._0.value.fetchEntities(collectionId, args_1);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      let _moved3 = false;
      const entities = _r2.unwrap();
      try {
        _moved3 = true;
        return Result.Ok([...entities].map((e) => R.fromEntity(e)));
      } finally {
        if (!_moved3) dropOwned(entities);
      }
    } finally {
      if (!_moved1) args_1.drop();
    }
  }

  async fetchOne<R extends View & Clone>(args: TryInto): Promise<Result<R | null, RetrievalError>> {
    const _r0 = await this.fetch(args);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const views = _r0.unwrap();
    return Result.Ok(iterFirst([...views]));
  }

  query<R>(args: TryInto): Result<LiveQuery<R>, RetrievalError> {
    const _r0 = args.tryInto().mapErr((e) => e);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const args_1 = _r0.unwrap();
    try {
      const _b2 = R.Model.collection();
      _moved1 = true;
      const _r3 = this._0.value.query(_b2, args_1);
      if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
      return Result.Ok(_r3.unwrap().map());
    } finally {
      if (!_moved1) args_1.drop();
    }
  }

  async queryWait<R>(args: TryInto): Promise<Result<LiveQuery<R>, RetrievalError>> {
    const _r0 = this.query(args);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const livequery = _r0.unwrap();
    try {
      await livequery.waitInitialized();
      _moved1 = true;
      return Result.Ok(livequery);
    } finally {
      if (!_moved1) livequery.drop();
    }
  }

  async collection(id: CollectionId): Promise<Result<StorageCollectionWrapper, RetrievalError>> {
    return await this._0.value.collection(id);
  }

  clone(): Context {
    return new Context(this._0.clone());
  }
}

export class NodeAndContext<SE extends StorageEngine, PA extends PolicyAgent> extends Struct implements TContext {
  readonly node: Node<SE, PA>;
  readonly cdata: ContextData;

  constructor(node: Node<SE, PA>, cdata: ContextData) {
    super();
    this.node = node;
    this.cdata = cdata;
  }

  async getEntity(collectionId: CollectionId, id: EntityId, cached: boolean): Promise<Result<Entity, RetrievalError>> {
    tracing.debug(`Node(${this.node.deref().value.id}).get_entity ${id}-${collectionId.debug()}`);
    if (!this.node.deref().value.durable) {
      const _v = await this.node.getFromPeer(collectionId, [id], this.cdata);
      if (_v.isOk()) {
        const _v1 = _v.unwrap();
        [];
      } else {
        const _v2 = _v.unwrapErr();
        _arm1: {
          if (_v2.is('NoDurablePeers')) {
            const _v3 = _v2;
            let _g2;
            try {
              _g2 = cached;
            } catch (_e) {
              if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
              _v3.drop();
              throw _e;
            }
            if (_g2) {
              try {
                [];
              } finally {
                _v3.drop();
              }
              break _arm1;
            }
          }
          {
            const e = _v2;
            let _moved0 = false;
            try {
              {
                _moved0 = true;
                return Result.Err(e);
              }
            } finally {
              if (!_moved0) e.drop();
            }
          }
        }
      }
    }
    {
      const _v4 = this.node.deref().value.entities.get(id);
      if (_v4 != null) {
        const local = _v4;
        tracing.debug(`Node(${this.node.deref().value.id}).get_entity found local entity - returning`);
        return Result.Ok(local);
      }
    }
    tracing.debug(`${this.node}.get_entity fetching from storage`);
    const _r3 = await this.node.deref().value.collections.get(collectionId);
    if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
    const collection = _r3.unwrap();
    try {
      const _v5 = await collection.deref().value.getState(id);
      if (_v5.isOk()) {
        const entityState = _v5.unwrap();
        try {
          {
            const retriever = EphemeralNodeRetriever.new(collectionId.clone(), this.node, this.cdata);
            try {
              let _moved5 = false;
              const _b4 = collectionId.clone();
              try {
                const _b6 = entityState.payload.takeField('state');
                const _r7 = await this.node.deref().value.entities.withState(retriever, id, _b4, _b6);
                if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
                _moved5 = true;
                const [_changed, entity] = _r7.unwrap();
                return Result.Ok(entity);
              } finally {
                if (!_moved5) dropOwned(_b4);
              }
            } finally {
              retriever.drop();
            }
          }
        } finally {
          entityState.drop();
        }
      } else {
        const e = _v5.unwrapErr();
        return Result.Err(e);
      }
    } finally {
      collection.drop();
    }
  }

  async fetchEntities(collectionId: CollectionId, args: MatchArgs): Promise<Result<Entity[], RetrievalError>> {
    try {
      const _r0 = this.node.deref().value.policyAgent.canAccessCollection(this.cdata, collectionId);
      if (_r0.isErr()) return Result.Err(RetrievalError.fromAccessDenied(_r0.unwrapErr()));
      _r0.drop();
      const _r2 = this.node.deref().value.policyAgent.filterPredicate(this.cdata, collectionId, args.selection.takeField('predicate'));
      if (_r2.isErr()) return Result.Err(RetrievalError.fromAccessDenied(_r2.unwrapErr()));
      const _a1 = _r2.unwrap();
      args.selection.predicate.drop();
      args.selection.predicate = _a1;
      const _a3 = this.node.deref().value.typeResolver.resolveSelectionTypes(args.takeField('selection'));
      args.selection.drop();
      args.selection = _a3;
      if (!this.node.deref().value.durable) {
        const _r4 = await this.fetchFromPeer(collectionId, args.takeField('selection'));
        if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
        return Result.Ok(_r4.unwrap());
      } else {
        const _r5 = await this.node.deref().value.collections.get(collectionId);
        if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
        const storageCollection = _r5.unwrap();
        try {
          const _r6 = await storageCollection.deref().value.fetchStates(args.selection);
          if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
          let _moved7 = false;
          const states = _r6.unwrap();
          try {
            let _moved8 = false;
            let entities = [];
            try {
              _moved7 = true;
              const _seq13 = states;
              let _at14 = 0;
              try {
                while (_at14 < _seq13.length) {
                  const state = _seq13[_at14++];
                  try {
                    const retriever = EphemeralNodeRetriever.new(collectionId.clone(), this.node, this.cdata);
                    try {
                      let _moved10 = false;
                      const _b9 = collectionId.clone();
                      try {
                        const _b11 = state.payload.takeField('state');
                        const _r12 = await this.node.deref().value.entities.withState(retriever, state.payload.entityId, _b9, _b11);
                        if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
                        _moved10 = true;
                        const [, entity] = _r12.unwrap();
                        entities.push(entity);
                      } finally {
                        if (!_moved10) dropOwned(_b9);
                      }
                    } finally {
                      retriever.drop();
                    }
                  } finally {
                    state.drop();
                  }
                }
              } finally {
                dropOwned(_seq13.slice(_at14));
              }
              _moved8 = true;
              return Result.Ok(entities);
            } finally {
              if (!_moved8) dropOwned(entities);
            }
          } finally {
            if (!_moved7) dropOwned(states);
          }
        } finally {
          storageCollection.drop();
        }
      }
    } finally {
      args.drop();
    }
  }

  async commitLocalTrx(trx: Transaction): Promise<Result<void, MutationError>> {
    let _c1;
    const _t0 = unsupported('`compare_exchange` WRITES what the `Arc<AtomicBool>` holds, and it is reached through an accessor that hands out the value rather than the place');
    try {
      _c1 = _t0.isErr();
    } finally {
      _t0.drop();
    }
    if (_c1) {
      return Result.Err(new MutationError('General', { _0: 'Transaction already committed or rolled back' }));
    }
    let _moved2 = false;
    const trxId = trx.id.clone();
    try {
      let _moved3 = false;
      let entityEvents: [Entity, Event][] = [];
      try {
        for (const entity of trx.entities.iter()) {
          const _r4 = entity.generateCommitEvent();
          if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
          {
            const _v = _r4.unwrap();
            if (_v != null) {
              const event = _v;
              let _moved5 = false;
              try {
                if (event.isEntityCreate()) {
                  const createdIds = trx.createdEntityIds.read();
                  try {
                    if (!createdIds.value.has(entity.deref().id)) {
                      return Result.Err(new MutationError('General', { _0: `Cannot commit phantom entity ${entity.deref().id}: entity has empty parent (creation event) but was not created in this transaction via create()` }));
                    }
                  } finally {
                    createdIds.drop();
                  }
                }
                const _b6 = entity.clone();
                _moved5 = true;
                entityEvents.push([_b6, event]);
              } finally {
                if (!_moved5) event.drop();
              }
            }
          }
        }
        let attestedEvents = [];
        try {
          let _moved7 = false;
          let entityAttestedEvents: [Entity, Attested<Event>][] = [];
          try {
            _moved3 = true;
            const _seq14 = entityEvents;
            let _at15 = 0;
            try {
              while (_at15 < _seq14.length) {
                const [entity, event] = _seq14[_at15++];
                let _moved8 = false;
                try {
                  try {
                    const trxAlive = Arc.new(true);
                    const forked = entity.snapshot(trxAlive);
                    try {
                      const entityBefore = entity.deref().kind.match({
                        Transacted: (v) => {
                          const upstream = v.upstream;
                          return upstream.clone();
                        },
                        Primary: () => entity.clone(),
                      });
                      try {
                        const collectionId = event.collection;
                        try {
                          const retriever = EphemeralNodeRetriever.new(collectionId.clone(), this.node, this.cdata);
                          try {
                            const _r9 = await forked.applyEvent(retriever, event);
                            if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
                            _r9.drop();
                            const _r10 = this.node.deref().value.policyAgent.checkEvent(this.node, this.cdata, entityBefore, forked, event);
                            if (_r10.isErr()) return Result.Err(MutationError.fromAccessDenied(_r10.unwrapErr()));
                            let _moved11 = false;
                            const attestation = _r10.unwrap();
                            try {
                              const _b12 = event.clone();
                              _moved11 = true;
                              let _moved13 = false;
                              const attested = Attested.opt(_b12, attestation);
                              try {
                                attestedEvents.push(attested.clone());
                                _moved8 = true;
                                _moved13 = true;
                                entityAttestedEvents.push([entity, attested]);
                              } finally {
                                if (!_moved13) attested.drop();
                              }
                            } finally {
                              if (!_moved11) dropOwned(attestation);
                            }
                          } finally {
                            retriever.drop();
                          }
                        } finally {
                          collectionId.drop();
                        }
                      } finally {
                        entityBefore.drop();
                      }
                    } finally {
                      forked.drop();
                    }
                  } finally {
                    event.drop();
                  }
                } finally {
                  if (!_moved8) entity.drop();
                }
              }
            } finally {
              dropOwned(_seq14.slice(_at15));
            }
            for (const [entity, attestedEvent] of entityAttestedEvents) {
              const _r16 = await this.node.deref().value.collections.get(attestedEvent.payload.collection);
              if (_r16.isErr()) return Result.Err(MutationError.fromRetrievalError(_r16.unwrapErr()));
              const collection = _r16.unwrap();
              try {
                const _r17 = await collection.deref().value.addEvent(attestedEvent);
                if (_r17.isErr()) return Result.Err(_r17.unwrapErr());
                _r17.drop();
                entity.commitHead(Clock.new([attestedEvent.payload.id()]));
              } finally {
                collection.drop();
              }
            }
            _moved2 = true;
            const _r18 = await this.node.relayToRequiredPeers(this.cdata, trxId, attestedEvents);
            if (_r18.isErr()) return Result.Err(_r18.unwrapErr());
            _r18.drop();
            let _moved19 = false;
            let changes = [];
            try {
              _moved7 = true;
              const _seq35 = entityAttestedEvents;
              let _at36 = 0;
              try {
                while (_at36 < _seq35.length) {
                  const [entity, attestedEvent] = _seq35[_at36++];
                  let _moved20 = false;
                  let _moved21 = false;
                  try {
                    try {
                      const collectionId = attestedEvent.payload.collection;
                      const _r22 = await this.node.deref().value.collections.get(collectionId);
                      if (_r22.isErr()) return Result.Err(MutationError.fromRetrievalError(_r22.unwrapErr()));
                      const collection = _r22.unwrap();
                      try {
                        const _m24 = await (async () => {
                          return await (entity.deref().kind.match<any>({
                            Transacted: async (v) => {
                              const upstream = v.upstream;
                              const retriever = EphemeralNodeRetriever.new(collectionId.clone(), this.node, this.cdata);
                              try {
                                const _r23 = await upstream.applyEvent(retriever, attestedEvent.payload);
                                if (_r23.isErr()) return { $jump: 'return', $value: Result.Err(_r23.unwrapErr()) };
                                _r23.drop();
                                return upstream.clone();
                              } finally {
                                retriever.drop();
                              }
                            },
                            Primary: async () => {
                              _moved20 = true;
                              return entity;
                            },
                          }));
                        })();
                        if ((_m24 as any)?.$jump === 'return') return (_m24 as any).$value;
                        let _moved25 = false;
                        const canonicalEntity = (_m24 as any);
                        try {
                          const _r26 = canonicalEntity.toState();
                          if (_r26.isErr()) return Result.Err(MutationError.fromStateError(_r26.unwrapErr()));
                          let _moved27 = false;
                          const state = _r26.unwrap();
                          try {
                            const _b28 = canonicalEntity.id();
                            const _b29 = canonicalEntity.collection().clone();
                            _moved27 = true;
                            let _moved30 = false;
                            const entityState = new EntityState(_b28, _b29, state);
                            try {
                              let _moved31 = false;
                              const attestation = this.node.deref().value.policyAgent.attestState(this.node, entityState);
                              try {
                                _moved30 = true;
                                _moved31 = true;
                                let _moved32 = false;
                                const attested = Attested.opt(entityState, attestation);
                                try {
                                  _moved32 = true;
                                  const _r33 = await collection.deref().value.setState(attested);
                                  if (_r33.isErr()) return Result.Err(_r33.unwrapErr());
                                  _r33.drop();
                                  _moved25 = true;
                                  _moved21 = true;
                                  const _r34 = EntityChange.new(canonicalEntity, [attestedEvent]);
                                  if (_r34.isErr()) return Result.Err(_r34.unwrapErr());
                                  changes.push(_r34.unwrap());
                                } finally {
                                  if (!_moved32) attested.drop();
                                }
                              } finally {
                                if (!_moved31) dropOwned(attestation);
                              }
                            } finally {
                              if (!_moved30) entityState.drop();
                            }
                          } finally {
                            if (!_moved27) state.drop();
                          }
                        } finally {
                          if (!_moved25) canonicalEntity.drop();
                        }
                      } finally {
                        collection.drop();
                      }
                    } finally {
                      if (!_moved21) attestedEvent.drop();
                    }
                  } finally {
                    if (!_moved20) entity.drop();
                  }
                }
              } finally {
                dropOwned(_seq35.slice(_at36));
              }
              _moved19 = true;
              await this.node.deref().value.reactor.notifyChange(changes);
              return Result.Ok([]);
            } finally {
              if (!_moved19) dropOwned(changes);
            }
          } finally {
            if (!_moved7) dropOwned(entityAttestedEvents);
          }
        } finally {
          dropOwned(attestedEvents);
        }
      } finally {
        if (!_moved3) dropOwned(entityEvents);
      }
    } finally {
      if (!_moved2) trxId.drop();
    }
  }

  async fetchFromPeer(collectionId: CollectionId, selection: Selection): Promise<Result<Entity[], RetrievalError>> {
    let _moved0 = false;
    try {
      const _m1 = this.node.getDurablePeerRandom();
      const _m2 = new RetrievalError('NoDurablePeers', {});
      const _r3 = (_m1 != null ? (_m2.drop(), Result.Ok(_m1!)) : Result.Err(_m2));
      if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
      const peerId = _r3.unwrap();
      const _r4 = await this.node.fetchEntitiesFromLocal(collectionId, selection);
      if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
      const knownMatchedEntities = _r4.unwrap();
      try {
        const knownMatches = unsupported('`collect` builds whatever its target type names, and the engine could not name the type this one is collected into');
        const selectionClone = selection.clone();
        try {
          _moved0 = true;
          const _r6 = await this.node.request(peerId, this.cdata, new NodeRequestBody('Fetch', { collection: collectionId.clone(), selection: selection, knownMatches: knownMatches }));
          if (_r6.isErr()) return Result.Err(RetrievalError.fromRequestError(_r6.unwrapErr()));
          return await (_r6.unwrap().intoMatch({
            Fetch: async (v) => {
              const deltas = v._0;
              let _moved7 = false;
              try {
                const retriever = EphemeralNodeRetriever.new(collectionId.clone(), this.node, this.cdata);
                try {
                  _moved7 = true;
                  const _r8 = await NodeApplier.applyDeltas(this.node, peerId, deltas, retriever);
                  if (_r8.isErr()) return Result.Err(RetrievalError.fromApplyError(_r8.unwrapErr()));
                  _r8.drop();
                  return await this.node.fetchEntitiesFromLocal(collectionId, selectionClone);
                } finally {
                  retriever.drop();
                }
              } finally {
                if (!_moved7) dropOwned(deltas);
              }
            },
            Error: async (v) => {
              const e = v._0;
              tracing.debug(`Error from peer fetch: ${e}`);
              return Result.Err(new RetrievalError('Other', { _0: `${debugString(e)}` }));
            },
            CommitComplete: (v) => {
              try {
                tracing.debug('Unexpected response type from peer fetch');
                return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
              } finally {
                dropUnbound(v, []);
              }
            },
            Get: (v) => {
              try {
                tracing.debug('Unexpected response type from peer fetch');
                return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
              } finally {
                dropUnbound(v, []);
              }
            },
            GetEvents: (v) => {
              try {
                tracing.debug('Unexpected response type from peer fetch');
                return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
              } finally {
                dropUnbound(v, []);
              }
            },
            QuerySubscribed: (v) => {
              try {
                tracing.debug('Unexpected response type from peer fetch');
                return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
              } finally {
                dropUnbound(v, []);
              }
            },
            Success: () => {
              tracing.debug('Unexpected response type from peer fetch');
              return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
            },
          }));
        } finally {
          selectionClone.drop();
        }
      } finally {
        dropOwned(knownMatchedEntities);
      }
    } finally {
      if (!_moved0) selection.drop();
    }
  }

  nodeId(): EntityId {
    return this.node.deref().value.id;
  }

  createEntity(collection: CollectionId, trxAlive: Arc<boolean>): Entity {
    let _moved0 = false;
    try {
      const primaryEntity = this.node.deref().value.entities.create(collection);
      try {
        _moved0 = true;
        return primaryEntity.snapshot(trxAlive);
      } finally {
        primaryEntity.drop();
      }
    } finally {
      if (!_moved0) trxAlive.drop();
    }
  }

  checkWrite(entity: Entity): Result<void, AccessDenied> {
    return this.node.deref().value.policyAgent.checkWrite(this.cdata, entity, null);
  }

  getResidentEntity(id: EntityId): Entity | null {
    return this.node.deref().value.entities.get(id);
  }

  query(collectionId: CollectionId, args: MatchArgs): Result<EntityLiveQuery, RetrievalError> {
    let _moved0 = false;
    let _moved1 = false;
    try {
      try {
        const _b2 = this.cdata.clone();
        _moved0 = true;
        _moved1 = true;
        return EntityLiveQuery.new(this.node, collectionId, args, _b2);
      } finally {
        if (!_moved1) args.drop();
      }
    } finally {
      if (!_moved0) collectionId.drop();
    }
  }

  async collection(id: CollectionId): Promise<Result<StorageCollectionWrapper, RetrievalError>> {
    return await this.node.deref().value.system.collection(id);
  }
}

export interface TContext {
  nodeId(): EntityId;
  createEntity(collection: CollectionId, trxAlive: Arc<boolean>): Entity;
  checkWrite(entity: Entity): Result<void, AccessDenied>;
  getEntity(id: EntityId, collection: CollectionId, cached: boolean): Promise<Result<Entity, RetrievalError>>;
  getResidentEntity(id: EntityId): Entity | null;
  fetchEntities(collection: CollectionId, args: MatchArgs): Promise<Result<Entity[], RetrievalError>>;
  commitLocalTrx(trx: Transaction): Promise<Result<void, MutationError>>;
  query(collectionId: CollectionId, args: MatchArgs): Result<EntityLiveQuery, RetrievalError>;
  collection(id: CollectionId): Promise<Result<StorageCollectionWrapper, RetrievalError>>;
}

