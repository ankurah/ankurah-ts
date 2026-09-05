// MIRRORS: ankurah/core/src/storage.rs
import { Struct, Result, Arc, dropOwned } from '@ankurah/base';
import { Attested, CollectionId, EntityId, EntityState, Event, EventId } from '@ankurah/proto';

export class StorageCollectionWrapper extends Struct {
  _0: Arc<StorageCollection>;

  constructor(_0: Arc<StorageCollection>) {
    super();
    this._0 = _0;
  }

  static new(bucket: Arc<StorageCollection>): StorageCollectionWrapper {
    return new StorageCollectionWrapper(bucket);
  }

  deref(): Arc<StorageCollection> {
    return this._0;
  }

  clone(): StorageCollectionWrapper {
    return new StorageCollectionWrapper(this._0.clone());
  }
}

export interface StorageEngine {
  collection(id: CollectionId): Promise<Result<Arc<StorageCollection>, RetrievalError>>;
  deleteAllCollections(): Promise<Result<boolean, MutationError>>;
}

export abstract class StorageCollection {
  abstract setState(state: Attested<EntityState>): Promise<Result<boolean, MutationError>>;
  abstract getState(id: EntityId): Promise<Result<Attested<EntityState>, RetrievalError>>;
  abstract fetchStates(selection: Selection): Promise<Result<Attested<EntityState>[], RetrievalError>>;
  async setStates(states: Attested<EntityState>[]): Promise<Result<void, MutationError>> {
    const _seq1 = states;
    let _at2 = 0;
    try {
      while (_at2 < _seq1.length) {
        const state = _seq1[_at2++];
        const _r0 = await this.setState(state);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        _r0.drop();
      }
    } finally {
      dropOwned(_seq1.slice(_at2));
    }
    return Result.Ok([]);
  }
  async getStates(ids: EntityId[]): Promise<Result<Attested<EntityState>[], RetrievalError>> {
    let states = [];
    for (const id of ids) {
      const _v = await this.getState(id);
      if (_v.isOk()) {
        const state = _v.unwrap();
        return states.push(state);
      } else {
        const e = _v.unwrapErr();
        return Result.Err(e);
      }
    }
    return Result.Ok(states);
  }
  abstract addEvent(entityEvent: Attested<Event>): Promise<Result<boolean, MutationError>>;
  abstract getEvents(eventIds: EventId[]): Promise<Result<Attested<Event>[], RetrievalError>>;
  abstract dumpEntityEvents(id: EntityId): Promise<Result<Attested<Event>[], RetrievalError>>;
}

export function stateName(name: string): string {
  return `${name}_state`;
}

export function eventName(name: string): string {
  return `${name}_event`;
}

