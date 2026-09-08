// MIRRORS: ankurah/core/src/retrieval.rs
import { Struct, Result, Arc, Mutex, dropOwned, dropUnbound, HashMap, HashSet } from '@ankurah/base';
import { Attested, Clock, EntityId, EntityState, Event, EventId, CollectionId, NodeRequestBody } from '@ankurah/proto';
import { MutationError, RetrievalError } from './error';
import { Node } from './node';
import { StorageCollectionWrapper } from './storage';

export class LocalRetriever extends Struct implements GetEvents, Retrieve {
  _0: Arc<LocalRetrieverInner>;

  constructor(_0: Arc<LocalRetrieverInner>) {
    super();
    this._0 = _0;
  }

  static new(collection: StorageCollectionWrapper): LocalRetriever {
    let _moved0 = false;
    try {
      const _b1 = new Mutex(new HashMap<EventId, [Attested<Event>, boolean]>());
      _moved0 = true;
      return new LocalRetriever(Arc.new(new LocalRetrieverInner(collection, _b1)));
    } finally {
      if (!_moved0) collection.drop();
    }
  }

  async storeUsedEvents(): Promise<Result<void, RetrievalError>> {
    const _t0 = this._0.value.stagedEvents.lock();
    try {
      const staged = _t0.value.take();
      _t0.drop();
      {
        const _v = staged;
        if (_v != null) {
          const staged = _v;
          try {
            for (const [_id, [event, used]] of [...staged]) {
              if (used) {
                const _r1 = await this._0.value.collection.deref().value.addEvent(event);
                if (_r1.isErr()) return Result.Err(RetrievalError.fromMutationError(_r1.unwrapErr()));
                _r1.drop();
              }
            }
          } finally {
            dropOwned(staged);
          }
        }
      }
      return Result.Ok([]);
    } finally {
      _t0.drop();
    }
  }

  async retrieveEvent(eventIds: EventId[]): Promise<Result<[number, Attested<Event>[]], RetrievalError>> {
    let _moved0 = false;
    try {
      let _moved1 = false;
      let events = [];
      try {
        _moved0 = true;
        let _moved2 = false;
        let eventIds_1 = HashSet.from([...eventIds]);
        try {
          const _t3 = this._0.value.stagedEvents.lock();
          try {
            {
              const _v1 = _t3.value;
              if (_v1 != null) {
                const staged = _v1;
                eventIds_1.retain((id) => {
                  {
                    const _v = staged.get(id);
                    if (_v != null) {
                      const [event, used] = _v;
                      events.push(event.clone());
                      used.value = true;
                      return false;
                    } else {
                    return true;
                  }
                  }
                });
              }
            }
          } finally {
            _t3.drop();
          }
          if (eventIds_1.size === 0) {
            _moved1 = true;
            return Result.Ok([0, events]);
          }
          _moved2 = true;
          const _r4 = await this._0.value.collection.deref().value.getEvents([...eventIds_1]);
          if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
          let _moved5 = false;
          const storedEvents = _r4.unwrap();
          try {
            _moved5 = true;
            events.push(...storedEvents);
            _moved1 = true;
            return Result.Ok([1, events]);
          } finally {
            if (!_moved5) dropOwned(storedEvents);
          }
        } finally {
          if (!_moved2) dropOwned(eventIds_1);
        }
      } finally {
        if (!_moved1) dropOwned(events);
      }
    } finally {
      if (!_moved0) dropOwned(eventIds);
    }
  }

  stageEvents(events: Attested<Event>[]): void {
    let staged = this._0.value.stagedEvents.lock();
    try {
      const staged_1 = staged.value.getOrInsertWith(() => new HashMap());
      for (const event of [...events]) {
        let _moved1 = false;
        const _b0 = event.payload.id();
        try {
          const _b2 = [event, false];
          _moved1 = true;
          staged_1.set(_b0, _b2);
        } finally {
          if (!_moved1) dropOwned(_b0);
        }
      }
    } finally {
      staged.drop();
    }
  }

  markEventUsed(eventId: EventId): void {
    let staged = this._0.value.stagedEvents.lock();
    try {
      const staged_1 = staged.value.getOrInsertWith(() => new HashMap());
      const _m0 = staged_1.get(eventId);
      (_m0 != null ? (([, used]) => {
        used.value = true;
      })(_m0!) : null);
    } finally {
      staged.drop();
    }
  }

  async getState(entityId: EntityId): Promise<Result<Attested<EntityState> | null, RetrievalError>> {
    const _v = await this._0.value.collection.deref().value.getState(entityId);
    if (_v.isOk()) {
      const state = _v.unwrap();
      return Result.Ok(state);
    } else {
      const _v1 = _v.unwrapErr();
      if (_v1.is('EntityNotFound')) {
        const _v2 = _v1;
        try {
          return Result.Ok(null);
        } finally {
          _v2.drop();
        }
      }
      {
        const e = _v1;
        return Result.Err(e);
      }
    }
  }

  clone(): LocalRetriever {
    return new LocalRetriever(this._0.clone());
  }
}

class LocalRetrieverInner extends Struct {
  collection: StorageCollectionWrapper;
  stagedEvents: Mutex<HashMap<EventId, [Attested<Event>, boolean]> | null>;

  constructor(collection: StorageCollectionWrapper, stagedEvents: Mutex<HashMap<EventId, [Attested<Event>, boolean]> | null>) {
    super();
    this.collection = collection;
    this.stagedEvents = stagedEvents;
  }
}

export class EphemeralNodeRetriever<SE extends StorageEngine, PA extends PolicyAgent, C extends Iterable<ContextData>> extends Struct implements GetEvents, Retrieve {
  readonly collection: CollectionId;
  readonly node: Node<SE, PA>;
  readonly cdata: C;
  stagedEvents: Mutex<HashMap<EventId, [Attested<Event>, boolean]> | null>;

  constructor(collection: CollectionId, node: Node<SE, PA>, cdata: C, stagedEvents: Mutex<HashMap<EventId, [Attested<Event>, boolean]> | null>) {
    super();
    this.collection = collection;
    this.node = node;
    this.cdata = cdata;
    this.stagedEvents = stagedEvents;
  }

  // A `&T` field is a borrow: dropping this releases the borrow and nothing
  // else, so the cascade must not walk it.
  protected override ownedFields(): unknown[] {
    return [this.collection, this.stagedEvents];
  }

  static new<SE, PA, C>(collection: CollectionId, node: Node<SE, PA>, cdata: C): EphemeralNodeRetriever<SE, PA, C> {
    let _moved0 = false;
    try {
      const _b1 = new Mutex(new HashMap<EventId, [Attested<Event>, boolean]>());
      _moved0 = true;
      return new EphemeralNodeRetriever(collection, node, cdata, _b1);
    } finally {
      if (!_moved0) collection.drop();
    }
  }

  async storeUsedEvents(): Promise<Result<void, MutationError>> {
    const _t0 = this.stagedEvents.lock();
    try {
      const staged = _t0.value.take();
      _t0.drop();
      {
        const _v = staged;
        if (_v != null) {
          const staged = _v;
          try {
            const _r1 = await this.node.deref().value.system.collection(this.collection);
            if (_r1.isErr()) return Result.Err(MutationError.fromRetrievalError(_r1.unwrapErr()));
            const collection = _r1.unwrap();
            try {
              for (const [_id, [event, used]] of [...staged]) {
                if (used) {
                  const _r2 = await collection.deref().value.addEvent(event);
                  if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
                  _r2.drop();
                }
              }
            } finally {
              collection.drop();
            }
          } finally {
            dropOwned(staged);
          }
        }
      }
      return Result.Ok([]);
    } finally {
      _t0.drop();
    }
  }

  async retrieveEvent(eventIds: EventId[]): Promise<Result<[number, Attested<Event>[]], RetrievalError>> {
    let _moved0 = false;
    try {
      let _moved1 = false;
      let events = [];
      try {
        _moved0 = true;
        let _moved2 = false;
        let eventIds_1 = HashSet.from([...eventIds]);
        try {
          const _t3 = this.stagedEvents.lock();
          try {
            {
              const _v1 = _t3.value;
              if (_v1 != null) {
                const staged = _v1;
                eventIds_1.retain((id) => {
                  {
                    const _v = staged.get(id);
                    if (_v != null) {
                      const [event, used] = _v;
                      events.push(event.clone());
                      used.value = true;
                      return false;
                    } else {
                    return true;
                  }
                  }
                });
              }
            }
          } finally {
            _t3.drop();
          }
          if (eventIds_1.size === 0) {
            _moved1 = true;
            return Result.Ok([0, events]);
          }
          const _r4 = await this.node.deref().value.system.collection(this.collection);
          if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
          const collection = _r4.unwrap();
          try {
            const _r5 = await collection.deref().value.getEvents([...[...eventIds_1]]);
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            const _seq8 = _r5.unwrap();
            let _at9 = 0;
            try {
              while (_at9 < _seq8.length) {
                const event = _seq8[_at9++];
                let _moved6 = false;
                try {
                  const _t7 = event.payload.id();
                  try {
                    eventIds_1.delete(_t7);
                  } finally {
                    _t7.drop();
                  }
                  _moved6 = true;
                  events.push(event);
                } finally {
                  if (!_moved6) event.drop();
                }
              }
            } finally {
              dropOwned(_seq8.slice(_at9));
            }
            if (eventIds_1.size === 0) {
              _moved1 = true;
              return Result.Ok([1, events]);
            }
            const _v2 = this.node.getDurablePeerRandom();
            if (!(_v2 != null)) {
              _moved1 = true;
              return Result.Ok([1, events]);
            }
            const peerId = _v2;
            _moved2 = true;
            const _r10 = await this.node.request(peerId, this.cdata, new NodeRequestBody('GetEvents', { collection: this.collection.clone(), eventIds: [...eventIds_1] }));
            if (_r10.isErr()) return Result.Err(RetrievalError.fromRequestError(_r10.unwrapErr()));
            const _m13 = await (_r10.unwrap().intoMatch<any>({
              GetEvents: async (v) => {
                const peerEvents = v._0;
                let _moved11 = false;
                try {
                  for (const event of [...peerEvents]) {
                    const _r12 = await collection.deref().value.addEvent(event);
                    if (_r12.isErr()) return { $jump: 'return', $value: Result.Err(RetrievalError.fromMutationError(_r12.unwrapErr())) };
                    _r12.drop();
                  }
                  _moved11 = true;
                  events.push(...peerEvents);
                } finally {
                  if (!_moved11) dropOwned(peerEvents);
                }
              },
              Error: async (v) => {
                const e = v._0;
                return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: `Error from peer: ${e}` })) };
              },
              CommitComplete: (v) => {
                try {
                  return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) };
                } finally {
                  dropUnbound(v, []);
                }
              },
              Fetch: (v) => {
                try {
                  return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) };
                } finally {
                  dropUnbound(v, []);
                }
              },
              Get: (v) => {
                try {
                  return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) };
                } finally {
                  dropUnbound(v, []);
                }
              },
              QuerySubscribed: (v) => {
                try {
                  return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) };
                } finally {
                  dropUnbound(v, []);
                }
              },
              Success: () => {
                return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) };
              },
            }));
            if ((_m13 as any)?.$jump === 'return') return (_m13 as any).$value;
            _moved1 = true;
            return Result.Ok([5, events]);
          } finally {
            collection.drop();
          }
        } finally {
          if (!_moved2) dropOwned(eventIds_1);
        }
      } finally {
        if (!_moved1) dropOwned(events);
      }
    } finally {
      if (!_moved0) dropOwned(eventIds);
    }
  }

  stageEvents(events: Attested<Event>[]): void {
    let staged = this.stagedEvents.lock();
    try {
      const staged_1 = staged.value.getOrInsertWith(() => new HashMap());
      for (const event of [...events]) {
        let _moved1 = false;
        const _b0 = event.payload.id();
        try {
          const _b2 = [event, false];
          _moved1 = true;
          staged_1.set(_b0, _b2);
        } finally {
          if (!_moved1) dropOwned(_b0);
        }
      }
    } finally {
      staged.drop();
    }
  }

  markEventUsed(eventId: EventId): void {
    let staged = this.stagedEvents.lock();
    try {
      const staged_1 = staged.value.getOrInsertWith(() => new HashMap());
      const _m0 = staged_1.get(eventId);
      (_m0 != null ? (([, used]) => {
        used.value = true;
      })(_m0!) : null);
    } finally {
      staged.drop();
    }
  }

  async getState(entityId: EntityId): Promise<Result<Attested<EntityState> | null, RetrievalError>> {
    const _r0 = await this.node.deref().value.collections.get(this.collection);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const collection = _r0.unwrap();
    try {
      const _v = await collection.deref().value.getState(entityId);
      if (_v.isOk()) {
        const state = _v.unwrap();
        return Result.Ok(state);
      } else {
        const _v1 = _v.unwrapErr();
        if (_v1.is('EntityNotFound')) {
          const _v2 = _v1;
          try {
            return Result.Ok(null);
          } finally {
            _v2.drop();
          }
        }
        {
          const e = _v1;
          return Result.Err(e);
        }
      }
    } finally {
      collection.drop();
    }
  }
}

export interface TEvent {
  id(): Id;
  parent(): Parent;
}

export interface TClock {
  members(): Id[];
}

export abstract class GetEvents {
  estimateCost(_batchSize: number): number {
    return 1;
  }
  abstract retrieveEvent(eventIds: Id[]): Promise<Result<[number, Attested<Event>[]], RetrievalError>>;
  abstract stageEvents(events: Attested<Event>[]): void;
  abstract markEventUsed(eventId: Id): void;
}

export interface Retrieve extends GetEvents {
  getState(entityId: EntityId): Promise<Result<Attested<EntityState> | null, RetrievalError>>;
}

export function Clock_members(self: Clock): EventId[] {
  return self.asSlice();
}

export function Event_id(self: Event): EventId {
  return self.id();
}

export function Event_parent(self: Event): Clock {
  return self.parent;
}

export function TEvent_dispatch_id(self: unknown): Id {
  if (self instanceof TestEvent) return (self as any).id();
  if (self instanceof Event) return Event_id(self as any);
  throw new Error(`BUG: no TEvent impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

export function TEvent_dispatch_parent(self: unknown): Parent {
  if (self instanceof TestEvent) return (self as any).parent();
  if (self instanceof Event) return Event_parent(self as any);
  throw new Error(`BUG: no TEvent impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

export function TClock_dispatch_members(self: unknown): Id[] {
  if (self instanceof TestClock) return (self as any).members();
  if (self instanceof Clock) return Clock_members(self as any);
  throw new Error(`BUG: no TClock impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

