// MIRRORS: ankurah/core/src/context.rs
import { Struct, Result, Arc, dropOwned, OwnershipFatal, UnsupportedShape, tracing, dropUnbound, unsupported } from '@ankurah/base';
import { Attested, Clock, CollectionId, EntityState, EntityId, NodeRequestBody } from '@ankurah/proto';
import { EntityChange } from './changes';
import { Entity } from './entity';
import { MutationError, RetrievalError } from './error';
import { Model, View } from './indexel';
import { EntityLiveQuery, LiveQuery } from './livequery';
import { ContextData, MatchArgs, Node } from './node';
import { NodeApplier } from './node_applier';
import { AccessDenied, PolicyAgent } from './policy';
import { EphemeralNodeRetriever, GetEvents } from './retrieval';
import { StorageCollectionWrapper, StorageEngine } from './storage';
import { Transaction } from './transaction';
import { Selection } from '@ankurah/ankql';
import { Get } from '@ankurah/signals';

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
    return Result.Ok([...views].next());
  }

  query<R>(args: TryInto): Result<LiveQuery<R>, RetrievalError> {
    const _r0 = args.tryInto().mapErr((e) => e);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const args_1 = _r0.unwrap();
    try {
      _moved1 = true;
      const _r2 = this._0.value.query(R.Model.collection(), args_1);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      return Result.Ok(_r2.unwrap().map());
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
            const _r4 = await this.node.deref().value.entities.withState(retriever, id, collectionId.clone(), entityState.payload.takeField('state'));
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            const [_changed, entity] = _r4.unwrap();
            return Result.Ok(entity);
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
            let entities = [];
            _moved7 = true;
            const _seq9 = states;
            let _at10 = 0;
            try {
              while (_at10 < _seq9.length) {
                const state = _seq9[_at10++];
                try {
                  const retriever = EphemeralNodeRetriever.new(collectionId.clone(), this.node, this.cdata);
                  const _r8 = await this.node.deref().value.entities.withState(retriever, state.payload.entityId, collectionId.clone(), state.payload.takeField('state'));
                  if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
                  const [, entity] = _r8.unwrap();
                  entities.push(entity);
                } finally {
                  state.drop();
                }
              }
            } finally {
              dropOwned(_seq9.slice(_at10));
            }
            return Result.Ok(entities);
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
    const _t0 = (() => { if (trx.alive.value === true) { trx.alive.value = false; return true; } return false; })();
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
      let entityEvents = [];
      for (const entity of trx.entities.iter()) {
        const _r3 = entity.generateCommitEvent();
        if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
        {
          const _v = _r3.unwrap();
          if (_v != null) {
            const event = _v;
            let _moved4 = false;
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
              _moved4 = true;
              entityEvents.push([entity.clone(), event]);
            } finally {
              if (!_moved4) event.drop();
            }
          }
        }
      }
      let attestedEvents = [];
      let entityAttestedEvents = [];
      for (const [entity, event] of entityEvents) {
        const trxAlive = Arc.new(true);
        const forked = entity.snapshot(trxAlive);
        const entityBefore = (() => {
          return entity.kind.match({
            Transacted: (v) => {
              const upstream = v.upstream;
              return upstream.clone();
            },
            Primary: () => entity.clone(),
          });
        })();
        const collectionId = event.collection;
        const retriever = EphemeralNodeRetriever.new(collectionId.clone(), this.node, this.cdata);
        const _r5 = await forked.applyEvent(retriever, event);
        if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
        _r5.drop();
        const _r6 = this.node.deref().value.policyAgent.checkEvent(this.node, this.cdata, entityBefore, forked, event);
        if (_r6.isErr()) return Result.Err(MutationError.fromAccessDenied(_r6.unwrapErr()));
        let _moved7 = false;
        const attestation = _r6.unwrap();
        try {
          _moved7 = true;
          const attested = Attested.opt(event.clone(), attestation);
          attestedEvents.push(attested.clone());
          entityAttestedEvents.push([entity, attested]);
        } finally {
          if (!_moved7) dropOwned(attestation);
        }
      }
      for (const [entity, attestedEvent] of entityAttestedEvents) {
        const _r8 = await this.node.deref().value.collections.get(attestedEvent.payload.collection);
        if (_r8.isErr()) return Result.Err(MutationError.fromRetrievalError(_r8.unwrapErr()));
        const collection = _r8.unwrap();
        try {
          const _r9 = await collection.deref().value.addEvent(attestedEvent);
          if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
          _r9.drop();
          entity.commitHead(Clock.new([attestedEvent.payload.id()]));
        } finally {
          collection.drop();
        }
      }
      _moved2 = true;
      const _r10 = await this.node.relayToRequiredPeers(this.cdata, trxId, attestedEvents);
      if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
      _r10.drop();
      let _moved11 = false;
      let changes = [];
      try {
        for (const [entity, attestedEvent] of entityAttestedEvents) {
          const collectionId = attestedEvent.payload.collection;
          const _r12 = await this.node.deref().value.collections.get(collectionId);
          if (_r12.isErr()) return Result.Err(MutationError.fromRetrievalError(_r12.unwrapErr()));
          const collection = _r12.unwrap();
          try {
            const _m14 = await (async () => {
              return await (entity.kind.match<any>({
                Transacted: async (v) => {
                  const upstream = v.upstream;
                  const retriever = EphemeralNodeRetriever.new(collectionId.clone(), this.node, this.cdata);
                  const _r13 = await upstream.applyEvent(retriever, attestedEvent.payload);
                  if (_r13.isErr()) return { $jump: 'return', $value: Result.Err(_r13.unwrapErr()) };
                  _r13.drop();
                  return upstream.clone();
                },
                Primary: async () => entity,
              }));
            })();
            if ((_m14 as any)?.$jump === 'return') return (_m14 as any).$value;
            const canonicalEntity = (_m14 as any);
            const _r15 = canonicalEntity.toState();
            if (_r15.isErr()) return Result.Err(_r15.unwrapErr());
            const state = _r15.unwrap();
            let _moved16 = false;
            const entityState = new EntityState(canonicalEntity.id(), canonicalEntity.collection().clone(), state);
            try {
              let _moved17 = false;
              const attestation = this.node.deref().value.policyAgent.attestState(this.node, entityState);
              try {
                _moved16 = true;
                _moved17 = true;
                const attested = Attested.opt(entityState, attestation);
                const _r18 = await collection.deref().value.setState(attested);
                if (_r18.isErr()) return Result.Err(_r18.unwrapErr());
                _r18.drop();
                const _r19 = EntityChange.new(canonicalEntity, [attestedEvent]);
                if (_r19.isErr()) return Result.Err(_r19.unwrapErr());
                changes.push(_r19.unwrap());
              } finally {
                if (!_moved17) dropOwned(attestation);
              }
            } finally {
              if (!_moved16) entityState.drop();
            }
          } finally {
            collection.drop();
          }
        }
        _moved11 = true;
        await this.node.deref().value.reactor.notifyChange(changes);
        return Result.Ok([]);
      } finally {
        if (!_moved11) dropOwned(changes);
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
                _moved7 = true;
                const _r8 = await NodeApplier.applyDeltas(this.node, peerId, deltas, retriever);
                if (_r8.isErr()) return Result.Err(RetrievalError.fromApplyError(_r8.unwrapErr()));
                _r8.drop();
                return await this.node.fetchEntitiesFromLocal(collectionId, selectionClone);
              } finally {
                if (!_moved7) dropOwned(deltas);
              }
            },
            Error: async (v) => {
              const e = v._0;
              tracing.debug(`Error from peer fetch: ${e}`);
              return Result.Err(new RetrievalError('Other', { _0: `${JSON.stringify(e)}` }));
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
    const primaryEntity = this.node.deref().value.entities.create(collection);
    try {
      return primaryEntity.snapshot(trxAlive);
    } finally {
      primaryEntity.drop();
    }
  }

  checkWrite(entity: Entity): Result<void, AccessDenied> {
    return this.node.deref().value.policyAgent.checkWrite(this.cdata, entity, null);
  }

  getResidentEntity(id: EntityId): Entity | null {
    return this.node.deref().value.entities.get(id);
  }

  query(collectionId: CollectionId, args: MatchArgs): Result<EntityLiveQuery, RetrievalError> {
    return EntityLiveQuery.new(this.node, collectionId, args, this.cdata.clone());
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

