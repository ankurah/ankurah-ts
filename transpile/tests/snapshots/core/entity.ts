// MIRRORS: ankurah/core/src/entity.rs
import { Struct, Enum, Result, Arc, Weak, RwLock, OwnedClosure, invoke, Invocable, dropOwned, valueNotEquals, tracing, dropUnbound, checkedAdd, iterFindMap, range, HashMap } from '@ankurah/base';
import { Clock, CollectionId, EntityId, EntityState, Event, EventId, OperationSet, State, Operation, StateBuffers } from '@ankurah/proto';
import { LineageError, MutationError, RetrievalError, StateError } from './error';
import { View } from './indexel';
import { compare, compareUnstoredEvent } from './lineage';
import { PropertyBackend, backendFromString } from './property/backend/index';
import { AbstractEntity } from './reactor';
import { State } from './reactor/subscription_state';
import { Filterable } from './selection/filter';
import { Value } from './value/index';
import { Broadcast } from '@ankurah/signals';

export class Entity extends Struct implements AbstractEntity, Filterable {
  _0: Arc<EntityInner>;

  constructor(_0: Arc<EntityInner>) {
    super();
    this._0 = _0;
  }

  id(): EntityId {
    return this.deref().id;
  }

  weak(): WeakEntity {
    return new WeakEntity(this._0.downgrade());
  }

  collection(): CollectionId {
    return this.deref().collection;
  }

  head(): Clock {
    const _t0 = this.deref().state.read();
    try {
      return _t0.value.head.clone();
    } finally {
      _t0.drop();
    }
  }

  isWritable(): boolean {
    return this.deref().kind.match({
      Primary: () => false,
      Transacted: (v) => {
        const trxAlive = v.trxAlive;
        return trxAlive.value;
      },
    });
  }

  toState(): Result<State, StateError> {
    const state = this.deref().state.read();
    try {
      let stateBuffers = new HashMap();
      for (const [name, backend] of state.value.backends) {
        const _r0 = backend.value.toStateBuffer();
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const stateBuffer = _r0.unwrap();
        stateBuffers.insert(name, stateBuffer);
      }
      const stateBuffers_1 = new StateBuffers(stateBuffers);
      return Result.Ok(new State(stateBuffers_1, state.value.head.clone()));
    } finally {
      state.drop();
    }
  }

  toEntityState(): Result<EntityState, StateError> {
    const _r0 = this.toState();
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const state = _r0.unwrap();
    try {
      _moved1 = true;
      return Result.Ok(new EntityState(this.id(), this.deref().collection.clone(), state));
    } finally {
      if (!_moved1) state.drop();
    }
  }

  static create(id: EntityId, collection: CollectionId): Entity {
    return new Entity(Arc.new(new EntityInner(id, collection, new RwLock(new EntityInnerState(Clock.default(), new HashMap<string, Arc<PropertyBackend>>())), new EntityKind('Primary', {}), Broadcast.new())));
  }

  static fromState(id: EntityId, collection: CollectionId, state: State): Result<Entity, RetrievalError> {
    let _moved0 = false;
    try {
      let backends = new HashMap();
      for (const [name, stateBuffer] of [...state.stateBuffers.deref()]) {
        const _r1 = backendFromString(name, stateBuffer);
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        let _moved2 = false;
        const backend = _r1.unwrap();
        try {
          _moved2 = true;
          backends.insert(name, backend);
        } finally {
          if (!_moved2) backend.drop();
        }
      }
      _moved0 = true;
      return Result.Ok(new Entity(Arc.new(new EntityInner(id, collection, new RwLock(new EntityInnerState(state.head.clone(), backends)), new EntityKind('Primary', {}), Broadcast.new()))));
    } finally {
      if (!_moved0) collection.drop();
    }
  }

  generateCommitEvent(): Result<Event | null, MutationError> {
    const state = this.deref().state.read();
    try {
      let _moved0 = false;
      let operations = new HashMap();
      try {
        for (const [name, backend] of state.value.backends) {
          const _r1 = backend.value.toOperations();
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          {
            const _v = _r1.unwrap();
            if (_v != null) {
              const ops = _v;
              operations.set(name, ops);
            }
          }
        }
        if (operations.size === 0) {
          return Result.Ok(null);
        } else {
          _moved0 = true;
          const operations_1 = new OperationSet(operations);
          _moved0 = true;
          const event = new Event(this.deref().collection.clone(), this.deref().id, operations_1, state.value.head.clone());
          return Result.Ok(event);
        }
      } finally {
        if (!_moved0) dropOwned(operations);
      }
    } finally {
      state.drop();
    }
  }

  commitHead(newHead: Clock): void {
    const _t0 = this.deref().state.write();
    try {
      const _a1 = newHead;
      _t0.value.head.drop();
      _t0.value.head = _a1;
    } finally {
      _t0.drop();
    }
  }

  tryMutate<E>(expectedHead: Clock, body: Invocable<[EntityInnerState], Result<void, E>>): Result<boolean, E> {
    let _moved0 = false;
    try {
      let state = this.deref().state.write();
      try {
        if (!state.value.head.equals(expectedHead)) {
          const _a1 = state.value.head.clone();
          expectedHead.value.drop();
          expectedHead.value = _a1;
          return Result.Ok(false);
        }
        _moved0 = true;
        const _r2 = invoke(body, state);
        if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
        _r2.drop();
        return Result.Ok(true);
      } finally {
        state.drop();
      }
    } finally {
      if (!_moved0) dropOwned(body);
    }
  }

  view<V extends View>(): V | null {
    if (valueNotEquals(this.collection(), V.collection())) {
      return null;
    } else {
      return V.fromEntity(this.clone());
    }
  }

  async applyEvent<G>(getter: G, event: Event): Promise<Result<boolean, MutationError>> {
    tracing.debug(`apply_event head: ${event} to ${this}`);
    if (event.isEntityCreate()) {
      let _moved0 = false;
      let state = this.deref().state.write();
      try {
        if (state.value.head.isEmpty()) {
          for (const [backendName, operations] of [...event.operations.deref()]) {
            const _r1 = state.value.applyOperations(backendName, operations);
            if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
            _r1.drop();
          }
          const _a2 = Clock.fromEventId(event.id());
          state.value.head.drop();
          state.value.head = _a2;
          _moved0 = true;
          state.drop();
          this.deref().broadcast.send([]);
          return Result.Ok(true);
        }
      } finally {
        if (!_moved0) state.drop();
      }
    }
    let head = this.head();
    try {
      const MAX_RETRIES = 5;
      const budget = 100;
      for (const attempt of range(0, MAX_RETRIES)) {
        const _r3 = await compareUnstoredEvent(getter, event, head, budget);
        if (_r3.isErr()) return { $jump: 'return', $value: Result.Err(MutationError.fromRetrievalError(_r3.unwrapErr())) };
        const _m5 = await (async () => {
          return _r3.unwrap().intoMatch<any>({
            Equal: () => {
              return { $jump: 'return', $value: Result.Ok(false) }
            },
            Descends: () => event.id(),
            NotDescends: (v) => {
              try {
                tracing.warn(`NotDescends - HACK - applying (attempt ${checkedAdd(attempt, 1, 'usize')})`);
                return head.withEvent(event.id());
              } finally {
                dropUnbound(v, []);
              }
            },
            Incomparable: () => {
              return { $jump: 'return', $value: Result.Err(MutationError.fromLineageError(new LineageError('Incomparable', {}))) };
            },
            PartiallyDescends: (v) => {
              const meet = v.meet;
              let _moved4 = false;
              try {
                _moved4 = true;
                return { $jump: 'return', $value: Result.Err(MutationError.fromLineageError(new LineageError('PartiallyDescends', { meet: meet }))) };
              } finally {
                if (!_moved4) dropOwned(meet);
              }
            },
            BudgetExceeded: (v) => {
              const subjectFrontier = v.subjectFrontier;
              const otherFrontier = v.otherFrontier;
              try {
                try {
                  tracing.warn(`apply_event budget exhausted after ${budget} events. Assuming Descends. subject_frontier: ${[...subjectFrontier].map((id) => id.toBase64Short()).join(', ')}, other_frontier: ${[...otherFrontier].map((id) => id.toBase64Short()).join(', ')}`);
                  return event.id();
                } finally {
                  dropOwned(otherFrontier);
                }
              } finally {
                dropOwned(subjectFrontier);
              }
            },
          });
        })();
        if ((_m5 as any)?.$jump === 'return') return (_m5 as any).$value;
        const newHead = (_m5 as any);
        let _c9;
        const _r8 = this.tryMutate(head, new OwnedClosure([newHead], (state: EntityInnerState) => {
          for (const [backendName, operations] of [...event.operations.deref()]) {
            const _r6 = state.applyOperations(backendName, operations);
            if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
            _r6.drop();
          }
          const _a7 = newHead;
          state.head.drop();
          state.head = _a7;
          return Result.Ok([]);
        }, undefined, true));
        if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
        _c9 = _r8.unwrap();
        if (_c9) {
          this.deref().broadcast.send([]);
          return Result.Ok(true);
        }
        continue;
      }
      tracing.warn('apply_event retries exhausted while chasing moving head; applying event as Descends');
      return Result.Err(new MutationError('TOCTOUAttemptsExhausted', {}));
    } finally {
      head.drop();
    }
  }

  async applyState<G>(getter: G, state: State): Promise<Result<boolean, MutationError>> {
    let head = this.head();
    try {
      const newHead = state.head.clone();
      try {
        tracing.debug(`${this} apply_state - new head: ${newHead}`);
        const budget = 100;
        const MAX_RETRIES = 5;
        for (const _attempt of range(0, MAX_RETRIES)) {
          const _r0 = await compare(getter, newHead, head, budget);
          if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(MutationError.fromRetrievalError(_r0.unwrapErr())) };
          const _m1 = await (async () => {
            return _r0.unwrap().intoMatch<any>({
              Equal: () => {
                return { $jump: 'return', $value: Result.Ok(false) }
              },
              Descends: () => true,
              NotDescends: (v) => {
                try {
                  return { $jump: 'return', $value: Result.Ok(false) }
                } finally {
                  dropUnbound(v, []);
                }
              },
              Incomparable: () => {
                return { $jump: 'return', $value: Result.Err(MutationError.fromLineageError(new LineageError('Incomparable', {}))) }
              },
              PartiallyDescends: (v) => {
                const meet = v.meet;
                return { $jump: 'return', $value: Result.Err(MutationError.fromLineageError(new LineageError('PartiallyDescends', { meet: meet }))) }
              },
              BudgetExceeded: (v) => {
                const subjectFrontier = v.subjectFrontier;
                const otherFrontier = v.otherFrontier;
                try {
                  try {
                    tracing.warn(`${this} apply_state - budget exhausted after ${budget} events. Assuming Descends. subject: ${subjectFrontier}, other: ${otherFrontier}`);
                    return true;
                  } finally {
                    dropOwned(otherFrontier);
                  }
                } finally {
                  dropOwned(subjectFrontier);
                }
              },
            });
          })();
          if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
          const apply = (_m1 as any);
          if (apply) {
            let _c5;
            const _r4 = this.tryMutate(head, (es) => {
              for (const [name, stateBuffer] of [...state.stateBuffers.deref()]) {
                const _r2 = backendFromString(name, stateBuffer);
                if (_r2.isErr()) return Result.Err(MutationError.fromRetrievalError(_r2.unwrapErr()));
                let _moved3 = false;
                const backend = _r2.unwrap();
                try {
                  _moved3 = true;
                  es.backends.insert(name, backend);
                } finally {
                  if (!_moved3) backend.drop();
                }
              }
              es.head = state.head.clone();
              return Result.Ok([]);
            });
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            _c5 = _r4.unwrap();
            if (_c5) {
              this.deref().broadcast.send([]);
              return Result.Ok(true);
            }
            continue;
          }
        }
        tracing.warn(`${this} apply_state retries exhausted while chasing moving head`);
        return Result.Err(new MutationError('TOCTOUAttemptsExhausted', {}));
      } finally {
        newHead.drop();
      }
    } finally {
      head.drop();
    }
  }

  snapshot(trxAlive: Arc<boolean>): Entity {
    const state = this.deref().state.read();
    try {
      let forked = new HashMap();
      for (const [name, backend] of state.value.backends) {
        forked.insert(name, backend.value.fork());
      }
      return new Entity(Arc.new(new EntityInner(this.deref().id, this.deref().collection.clone(), new RwLock(new EntityInnerState(state.value.head.clone(), forked)), new EntityKind('Transacted', { trxAlive: trxAlive, upstream: this.clone() }), Broadcast.new())));
    } finally {
      state.drop();
    }
  }

  broadcast(): Broadcast<void> {
    return this.deref().broadcast;
  }

  getBackend<P extends PropertyBackend>(): Result<Arc<P>, RetrievalError> {
    const backendName = P.propertyBackendName();
    let state = this.deref().state.write();
    try {
      {
        const _v = state.value.backends.get(backendName);
        if (_v != null) {
          const backend = _v;
          const _t0 = backend.clone();
          try {
            const upcasted = _t0.value.asArcDynAny();
            return Result.Ok(upcasted.downcast());
          } finally {
            _t0.drop();
          }
        } else {
        const _r1 = backendFromString(backendName, null);
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        let _moved2 = false;
        const backend = _r1.unwrap();
        try {
          const _t3 = backend.clone();
          try {
            const upcasted = _t3.value.asArcDynAny();
            const typedBackend = upcasted.downcast();
            _moved2 = true;
            state.value.backends.set(backendName, backend);
            return Result.Ok(typedBackend);
          } finally {
            _t3.drop();
          }
        } finally {
          if (!_moved2) backend.drop();
        }
      }
      }
    } finally {
      state.drop();
    }
  }

  values(): [string, Value | null][] {
    const state = this.deref().state.read();
    try {
      return state.value.backends.values().flatMap((backend) => {
        const _t0 = backend.value.propertyValues();
        try {
          return [..._t0].map(([name, value]) => [name.toString(), value.clone()]);
        } finally {
          dropOwned(_t0);
        }
      });
    } finally {
      state.drop();
    }
  }

  deref(): EntityInner {
    return this._0;
  }

  equals(other: Entity): boolean {
    return Arc.ptrEq(this._0, other._0);
  }

  value(field: string): Value | null {
    if (field === 'id') {
      return new Value('EntityId', { _0: this.deref().id });
    } else {
      const state = this.deref().state.read();
      try {
        return iterFindMap(state.value.backends.values(), (backend) => backend.value.propertyValue(field));
      } finally {
        state.drop();
      }
    }
  }

  toString(): string {
    return `Entity(${this.deref().collection}/${this.deref().id.toBase64Short()} ${this.head()})`;
  }

  clone(): Entity {
    return new Entity(this._0.clone());
  }

  debug(): string {
    return `Entity(${this._0.value.debug()})`;
  }
}

export class TemporaryEntity extends Struct implements Filterable {
  _0: Arc<EntityInner>;

  constructor(_0: Arc<EntityInner>) {
    super();
    this._0 = _0;
  }

  static new(id: EntityId, collection: CollectionId, state: State): Result<TemporaryEntity, RetrievalError> {
    let _moved0 = false;
    try {
      let backends = new HashMap();
      for (const [name, stateBuffer] of [...state.stateBuffers.deref()]) {
        const _r1 = backendFromString(name, stateBuffer);
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        let _moved2 = false;
        const backend = _r1.unwrap();
        try {
          _moved2 = true;
          backends.insert(name, backend);
        } finally {
          if (!_moved2) backend.drop();
        }
      }
      _moved0 = true;
      return Result.Ok(new TemporaryEntity(Arc.new(new EntityInner(id, collection, new RwLock(new EntityInnerState(state.head.clone(), backends)), new EntityKind('Primary', {}), Broadcast.new()))));
    } finally {
      if (!_moved0) collection.drop();
    }
  }

  values(): [string, Value | null][] {
    const state = this._0.value.state.read();
    try {
      return state.value.backends.values().flatMap((backend) => backend.value.propertyValues());
    } finally {
      state.drop();
    }
  }

  deref(): EntityInner {
    return this._0;
  }

  collection(): string {
    return this._0.value.collection.asStr();
  }

  value(name: string): Value | null {
    if (name === 'id') {
      return new Value('EntityId', { _0: this._0.value.id });
    } else {
      const state = this._0.value.state.read();
      try {
        return iterFindMap(state.value.backends.values(), (backend) => backend.value.propertyValue(name));
      } finally {
        state.drop();
      }
    }
  }

  toString(): string {
    const _t0 = this._0.value.state.read();
    try {
      return `TemporaryEntity(${this.deref().collection}/${this.deref().id}) = ${_t0.value.head}`;
    } finally {
      _t0.drop();
    }
  }
}

class EntityInnerState extends Struct {
  head: Clock;
  backends: HashMap<string, Arc<PropertyBackend>>;

  constructor(head: Clock, backends: HashMap<string, Arc<PropertyBackend>>) {
    super();
    this.head = head;
    this.backends = backends;
  }

  applyOperations(backendName: string, operations: Operation[]): Result<void, MutationError> {
    {
      const _v = this.backends.get(backendName);
      if (_v != null) {
        const backend = _v;
        const _r0 = backend.value.applyOperations(operations);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        _r0.drop();
      } else {
      const _r1 = backendFromString(backendName, null);
      if (_r1.isErr()) return Result.Err(MutationError.fromRetrievalError(_r1.unwrapErr()));
      let _moved2 = false;
      const backend = _r1.unwrap();
      try {
        const _r3 = backend.value.applyOperations(operations);
        if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
        _r3.drop();
        _moved2 = true;
        this.backends.set(backendName, backend);
      } finally {
        if (!_moved2) backend.drop();
      }
    }
    }
    return Result.Ok([]);
  }

  debug(): string {
    return `EntityInnerState { head: ${this.head}, backends: ${this.backends} }`;
  }
}

export class EntityInner extends Struct {
  readonly id: EntityId;
  readonly collection: CollectionId;
  state: RwLock<EntityInnerState>;
  kind: EntityKind;
  broadcast: Broadcast<void>;

  constructor(id: EntityId, collection: CollectionId, state: RwLock<EntityInnerState>, kind: EntityKind, broadcast: Broadcast<void>) {
    super();
    this.id = id;
    this.collection = collection;
    this.state = state;
    this.kind = kind;
    this.broadcast = broadcast;
  }

  debug(): string {
    return `EntityInner { id: ${this.id}, collection: ${this.collection.debug()}, state: ${this.state}, kind: ${this.kind.debug()}, broadcast: ${this.broadcast.debug()} }`;
  }
}

export class WeakEntity extends Struct {
  _0: Weak<EntityInner>;

  constructor(_0: Weak<EntityInner>) {
    super();
    this._0 = _0;
  }

  upgrade(): Entity | null {
    const _m0 = this._0.upgrade();
    return (_m0 != null ? (Entity)(_m0!) : null);
  }
}

export class WeakEntitySet extends Struct {
  _0: Arc<RwLock<HashMap<EntityId, WeakEntity>>>;

  constructor(_0: Arc<RwLock<HashMap<EntityId, WeakEntity>>>) {
    super();
    this._0 = _0;
  }

  get(id: EntityId): Entity | null {
    const entities = this._0.value.read();
    try {
      {
        const _v = entities.value.get(id);
        if (_v != null) {
          const entity = _v;
          return entity.upgrade();
        } else {
        return null;
      }
      }
    } finally {
      entities.drop();
    }
  }

  async getOrRetrieve<R>(retriever: R, collectionId: CollectionId, id: EntityId): Promise<Result<Entity | null, RetrievalError>> {
    const _v = this.get(id);
    if (_v != null) {
      const entity = _v;
      return Result.Ok(entity);
    } else {
      const _v1 = await retriever.getState(id);
      if (_v1.isOk()) {
        const _v2 = _v1.unwrap();
        if (_v2 == null) {
          const _v3 = _v2;
          try {
            return Result.Ok(null);
          } finally {
            dropOwned(_v3);
          }
        }
        {
          const state = _v2;
          try {
            {
              const _r0 = await this.withState(retriever, id, collectionId.clone(), state.payload.takeField('state'));
              if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
              const [, entity] = _r0.unwrap();
              return Result.Ok(entity);
            }
          } finally {
            state.drop();
          }
        }
      } else {
        const e = _v1.unwrapErr();
        return Result.Err(e);
      }
    }
  }

  async getRetrieveOrCreate<R>(retriever: R, collectionId: CollectionId, id: EntityId): Promise<Result<Entity, RetrievalError>> {
    const _r0 = await this.getOrRetrieve(retriever, collectionId, id);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const _v = _r0.unwrap();
    if (_v != null) {
      const entity = _v;
      return Result.Ok(entity);
    } else {
      {
        let entities = this._0.value.write();
        try {
          {
            const _v2 = entities.value.get(id);
            if (_v2 != null) {
              const entity = _v2;
              {
                const _v1 = entity.upgrade();
                if (_v1 != null) {
                  const entity = _v1;
                  return Result.Ok(entity);
                }
              }
            }
          }
          let _moved1 = false;
          const entity = Entity.create(id, collectionId.clone());
          try {
            entities.value.set(id, entity.weak());
            _moved1 = true;
            return Result.Ok(entity);
          } finally {
            if (!_moved1) entity.drop();
          }
        } finally {
          entities.drop();
        }
      }
    }
  }

  create(collection: CollectionId): Entity {
    let entities = this._0.value.write();
    try {
      const id = EntityId.new();
      const entity = Entity.create(id, collection);
      entities.value.set(id, entity.weak());
      return entity;
    } finally {
      entities.drop();
    }
  }

  privateGetOrCreate(id: EntityId, collectionId: CollectionId, state: State): Result<[boolean, Entity], RetrievalError> {
    let entities = this._0.value.write();
    try {
      {
        const _v1 = entities.value.get(id);
        if (_v1 != null) {
          const existingWeak = _v1;
          {
            const _v = existingWeak.upgrade();
            if (_v != null) {
              const existingEntity = _v;
              tracing.debug(`Entity ${id} was created by another thread during async work, using that one`);
              return Result.Ok([true, existingEntity]);
            }
          }
        }
      }
      const _r0 = Entity.fromState(id, collectionId.clone(), state);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      let _moved1 = false;
      const entity = _r0.unwrap();
      try {
        entities.value.set(id, entity.weak());
        _moved1 = true;
        return Result.Ok([false, entity]);
      } finally {
        if (!_moved1) entity.drop();
      }
    } finally {
      entities.drop();
    }
  }

  async withState<R>(retriever: R, id: EntityId, collectionId: CollectionId, state: State): Promise<Result<[boolean | null, Entity], RetrievalError>> {
    try {
      try {
        const _m4 = await (async () => {
          const _v = this.get(id);
          if (_v != null) {
            const entity = _v;
            return entity;
          } else {
            const _r0 = await retriever.getState(id);
            if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
            {
              const _v2 = _r0.unwrap();
              if (_v2 != null) {
                const storedState = _v2;
                try {
                  const _r1 = this.privateGetOrCreate(id, collectionId, storedState.payload.state);
                  if (_r1.isErr()) return { $jump: 'return', $value: Result.Err(_r1.unwrapErr()) };
                  const _t2 = _r1.unwrap();
                  try {
                    return _t2[1];
                  } finally {
                    dropOwned(_t2);
                  }
                } finally {
                  storedState.drop();
                }
              } else {
              const _r3 = this.privateGetOrCreate(id, collectionId, state);
              if (_r3.isErr()) return { $jump: 'return', $value: Result.Err(_r3.unwrapErr()) };
              const _v1 = _r3.unwrap();
              if ((_v1[0] === true)) {
                const entity = _v1[1];
                return entity;
              } else {
                const entity = _v1[1];
                {
                  return { $jump: 'return', $value: Result.Ok([null, entity]) };
                }
              }
            }
            }
          }
        })();
        if ((_m4 as any)?.$jump === 'return') return (_m4 as any).$value;
        let _moved5 = false;
        const entity = (_m4 as any);
        try {
          const _r6 = await entity.applyState(retriever, state);
          if (_r6.isErr()) return Result.Err(RetrievalError.fromMutationError(_r6.unwrapErr()));
          const changed = _r6.unwrap();
          _moved5 = true;
          return Result.Ok([changed, entity]);
        } finally {
          if (!_moved5) entity.drop();
        }
      } finally {
        state.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  clone(): WeakEntitySet {
    return new WeakEntitySet(this._0.clone());
  }

  static default(): WeakEntitySet {
    return new WeakEntitySet(Arc.new(new RwLock(new HashMap())));
  }
}

export type EntityKindV = {
  Primary: {};
  Transacted: { trxAlive: Arc<boolean>; upstream: Entity };
};

export class EntityKind extends Enum<EntityKindV> {

  debug(): string {
    return this.match({
      Primary: () => 'Primary',
      Transacted: (v) => `Transacted { trxAlive: ${v.trxAlive}, upstream: ${v.upstream.debug()} }`,
    });
  }
}

