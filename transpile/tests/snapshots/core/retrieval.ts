// MIRRORS: ankurah/core/src/retrieval.rs
import { Struct, Result, Arc, Mutex, dropOwned, dropUnbound, HashMap } from '@ankurah/base';
import { Attested, Clock, EntityId, EntityState, Event, EventId, CollectionId, NodeRequestBody } from '@ankurah/proto';
import { MutationError, RetrievalError } from './error';
import { Node } from './node';
import { StorageCollectionWrapper } from './storage';
import { Get } from '@ankurah/signals';

export class LocalRetriever extends Struct implements GetEvents, Retrieve {
  _0: Arc<LocalRetrieverInner>;

  constructor(_0: Arc<LocalRetrieverInner>) {
    super();
    this._0 = _0;
  }

  static new(collection: StorageCollectionWrapper): LocalRetriever {
    return new LocalRetriever(Arc.new(new LocalRetrieverInner(collection, new Mutex(new HashMap<EventId, [Attested<Event>, boolean]>()))));
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
    let events = [];
    let _moved0 = false;
    const eventIds_1 = [...eventIds];
    try {
      const _t1 = this._0.value.stagedEvents.lock();
      try {
        {
          const _v1 = _t1.value;
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
        _t1.drop();
      }
      if (eventIds_1.size === 0) {
        return Result.Ok([0, events]);
      }
      _moved0 = true;
      const _r2 = await this._0.value.collection.deref().value.getEvents([...eventIds_1]);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      let _moved3 = false;
      const storedEvents = _r2.unwrap();
      try {
        _moved3 = true;
        events.extend(storedEvents);
        return Result.Ok([1, events]);
      } finally {
        if (!_moved3) dropOwned(storedEvents);
      }
    } finally {
      if (!_moved0) dropOwned(eventIds_1);
    }
  }

  stageEvents(events: Attested<Event>[]): void {
    let staged = this._0.value.stagedEvents.lock();
    try {
      const staged_1 = staged.value.getOrInsertWith(() => new HashMap());
      for (const event of [...events]) {
        staged_1.set(event.payload.id(), [event, false]);
      }
    } finally {
      staged.drop();
    }
  }

  markEventUsed(eventId: EventId): void {
    let staged = this._0.value.stagedEvents.lock();
    try {
      const staged_1 = staged.value.getOrInsertWith(() => new HashMap());
      staged_1.get(eventId) != null ? (([, used]) => {
        used.value = true;
      })(staged_1.get(eventId)!) : null;
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
      const e = _v.unwrapErr();
      return Result.Err(e);
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
    return new EphemeralNodeRetriever(collection, node, cdata, new Mutex(new HashMap<EventId, [Attested<Event>, boolean]>()));
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
    let events = [];
    let _moved0 = false;
    const eventIds_1 = [...eventIds];
    try {
      const _t1 = this.stagedEvents.lock();
      try {
        {
          const _v1 = _t1.value;
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
        _t1.drop();
      }
      if (eventIds_1.size === 0) {
        return Result.Ok([0, events]);
      }
      const _r2 = await this.node.deref().value.system.collection(this.collection);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      const collection = _r2.unwrap();
      try {
        const _r3 = await collection.deref().value.getEvents([...[...eventIds_1]]);
        if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
        const _seq5 = _r3.unwrap();
        let _at6 = 0;
        try {
          while (_at6 < _seq5.length) {
            const event = _seq5[_at6++];
            const _t4 = event.payload.id();
            try {
              eventIds_1.delete(_t4);
            } finally {
              _t4.drop();
            }
            events.push(event);
          }
        } finally {
          dropOwned(_seq5.slice(_at6));
        }
        if (eventIds_1.size === 0) {
          return Result.Ok([1, events]);
        }
        const _v2 = this.node.getDurablePeerRandom();
        if (!(_v2 != null)) {
          return Result.Ok([1, events]);
        }
        const peerId = _v2;
        _moved0 = true;
        const _r7 = await this.node.request(peerId, this.cdata, new NodeRequestBody('GetEvents', { collection: this.collection.clone(), eventIds: [...eventIds_1] }));
        if (_r7.isErr()) return Result.Err(RetrievalError.fromRequestError(_r7.unwrapErr()));
        const _m10 = await (_r7.unwrap().intoMatch<any>({
          GetEvents: async (v) => {
            const peerEvents = v._0;
            let _moved8 = false;
            try {
              for (const event of [...peerEvents]) {
                const _r9 = await collection.deref().value.addEvent(event);
                if (_r9.isErr()) return { $jump: 'return', $value: Result.Err(RetrievalError.fromMutationError(_r9.unwrapErr())) };
                _r9.drop();
              }
              _moved8 = true;
              events.extend(peerEvents);
            } finally {
              if (!_moved8) dropOwned(peerEvents);
            }
          },
          Error: (v) => {
            const e = v._0;
            return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: `Error from peer: ${e}` })) };
          },
          CommitComplete: (v) => {
            try {
              return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) }
            } finally {
              dropUnbound(v, []);
            }
          },
          Fetch: (v) => {
            try {
              return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) }
            } finally {
              dropUnbound(v, []);
            }
          },
          Get: (v) => {
            try {
              return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) }
            } finally {
              dropUnbound(v, []);
            }
          },
          QuerySubscribed: (v) => {
            try {
              return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) }
            } finally {
              dropUnbound(v, []);
            }
          },
          Success: () => {
            return { $jump: 'return', $value: Result.Err(new RetrievalError('StorageError', { _0: 'Unexpected response type from peer' })) }
          },
        }));
        if ((_m10 as any)?.$jump === 'return') return (_m10 as any).$value;
        return Result.Ok([5, events]);
      } finally {
        collection.drop();
      }
    } finally {
      if (!_moved0) dropOwned(eventIds_1);
    }
  }

  stageEvents(events: Attested<Event>[]): void {
    let staged = this.stagedEvents.lock();
    try {
      const staged_1 = staged.value.getOrInsertWith(() => new HashMap());
      for (const event of [...events]) {
        staged_1.set(event.payload.id(), [event, false]);
      }
    } finally {
      staged.drop();
    }
  }

  markEventUsed(eventId: EventId): void {
    let staged = this.stagedEvents.lock();
    try {
      const staged_1 = staged.value.getOrInsertWith(() => new HashMap());
      staged_1.get(eventId) != null ? (([, used]) => {
        used.value = true;
      })(staged_1.get(eventId)!) : null;
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
        const e = _v.unwrapErr();
        return Result.Err(e);
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

export interface Retrieve {
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

