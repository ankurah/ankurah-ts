// MIRRORS: ankurah/core/src/system.rs
import { Struct, Result, Arc, RwLock, AnyhowError, serde_json, dropOwned, tracing, HashMap, Notify, spawn } from '@ankurah/base';
import { Attested, Clock, CollectionId, EntityState, Event, Item } from '@ankurah/proto';
import { CollectionSet } from './collectionset';
import { Entity, WeakEntitySet } from './entity';
import { MutationError, RetrievalError } from './error';
import { Node } from './node';
import { Property } from './property/index';
import { PropertyError } from './property/traits';
import { LWW } from './property/value/lww';
import { Reactor } from './reactor';
import { LocalRetriever } from './retrieval';
import { StorageCollectionWrapper } from './storage';
import { spawn } from './task';
import { Value } from './value/index';
import { Predicate, Selection } from '@ankurah/ankql';

export class SystemManager<SE extends StorageEngine, PA extends PolicyAgent> extends Struct {
  _0: Arc<Inner<SE, PA>>;

  constructor(_0: Arc<Inner<SE, PA>>) {
    super();
    this._0 = _0;
  }

  static new<SE, PA>(collections: CollectionSet<SE>, entities: WeakEntitySet, reactor: Reactor<Entity, Attested<Event>>, durable: boolean): SystemManager<SE, PA> {
    const me = new SystemManager(Arc.new(new Inner(collections, new RwLock(new HashMap()), entities, durable, new RwLock(null), new RwLock([]), OnceLock.new(), Notify.new(), new RwLock(false), Notify.new(), reactor, undefined /* PhantomData */)));
    ((me) => {
      spawn((async () => {
        {
          const _v = await me.loadSystemCatalog();
          if (_v.isErr()) {
            const e = _v.unwrapErr();
            tracing.error(`Failed to load system catalog: ${e}`);
          }
        }
      })());
    })(me.clone())
    return me;
  }

  root(): Attested<EntityState> | null {
    const _t0 = this._0.value.root.read();
    try {
      return _t0.value != null ? ((r) => r.clone())(_t0.value!) : null;
    } finally {
      _t0.drop();
    }
  }

  items(): Entity[] {
    const _t0 = this._0.value.items.read();
    try {
      return _t0.value.clone();
    } finally {
      _t0.drop();
    }
  }

  async collection(id: CollectionId): Promise<Result<StorageCollectionWrapper, RetrievalError>> {
    await this.waitLoaded();
    return await this._0.value.collectionset.get(id);
  }

  isSystemReady(): boolean {
    const _t0 = this._0.value.systemReady.read();
    try {
      return _t0.value;
    } finally {
      _t0.drop();
    }
  }

  async waitSystemReady(): Promise<void> {
    if (!this.isSystemReady()) {
      await this._0.value.systemReadyNotify.notified();
    }
  }

  async create(): Promise<Result<void, Error>> {
    if (!this._0.value.durable) {
      return Result.Err(AnyhowError.msg('Only durable nodes can create a new system'));
    }
    await this.waitLoaded();
    const _m0 = (() => {
      {
        const items = this._0.value.items.read();
        try {
          if (!(items.value.length === 0)) {
            return { $jump: 'return', $value: Result.Err(AnyhowError.msg('System root already exists')) };
          }
        } finally {
          items.drop();
        }
      }
    })();
    if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
    (_m0 as any)
    const collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID);
    try {
      const _r1 = await this._0.value.collectionset.get(collectionId);
      if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
      const storage = _r1.unwrap();
      try {
        let _moved2 = false;
        const systemEntity = this._0.value.entities.create(collectionId.clone());
        try {
          const lwwBackend = systemEntity.getBackend().expect('LWW Backend should exist');
          try {
            const _r3 = Item_intoValue(new Item('SysRoot', {}));
            if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
            lwwBackend.value.set('item', _r3.unwrap());
            const _r4 = systemEntity.generateCommitEvent();
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            const _r5 = _r4.unwrap() != null ? Result.Ok(_r4.unwrap()!) : Result.Err(AnyhowError.msg('Expected event'));
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            let _moved6 = false;
            const event = _r5.unwrap();
            try {
              const root = Clock.fromEventId(event.id());
              try {
                _moved6 = true;
                const _r7 = await storage.deref().value.addEvent(event);
                if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
                _r7.drop();
                systemEntity.commitHead(root.clone());
                const _r8 = systemEntity.toEntityState();
                if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
                let _moved9 = false;
                const attestedState = Attested.fromEntityState(_r8.unwrap());
                try {
                  const _r10 = await storage.deref().value.setState(attestedState.clone());
                  if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
                  _r10.drop();
                  let items = this._0.value.items.write();
                  try {
                    _moved2 = true;
                    items.value.push(systemEntity);
                    _moved9 = true;
                    const _t11 = this._0.value.root.write();
                    try {
                      _t11.value = attestedState;
                    } finally {
                      _t11.drop();
                    }
                    const _t12 = this._0.value.systemReady.write();
                    try {
                      _t12.value = true;
                    } finally {
                      _t12.drop();
                    }
                    this._0.value.systemReadyNotify.notifyWaiters();
                    return Result.Ok([]);
                  } finally {
                    items.drop();
                  }
                } finally {
                  if (!_moved9) attestedState.drop();
                }
              } finally {
                root.drop();
              }
            } finally {
              if (!_moved6) event.drop();
            }
          } finally {
            lwwBackend.drop();
          }
        } finally {
          if (!_moved2) systemEntity.drop();
        }
      } finally {
        storage.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  async joinSystem(state: Attested<EntityState>): Promise<Result<void, MutationError>> {
    let _moved0 = false;
    try {
      await this.waitLoaded();
      if (this._0.value.durable) {
        tracing.warn('Durable node attempted to join system - this is not allowed');
        return Result.Err(new MutationError('General', { _0: io.Error.other('Durable nodes cannot join an existing system') }));
      }
      let _moved1 = false;
      const rootState = this.root();
      try {
        _moved1 = true;
        {
          const _v = rootState;
          if (_v != null) {
            const root = _v;
            try {
              if (root.payload.state.head.equals(state.payload.state.head)) {
                undefined /* notice_info!("Found matching root - Node is part of the same system") */;
                const _t2 = this._0.value.systemReady.write();
                try {
                  _t2.value = true;
                } finally {
                  _t2.drop();
                }
                this._0.value.systemReadyNotify.notifyWaiters();
                return Result.Ok([]);
              }
              tracing.warn(`Mismatched root state during join: local=${root.debug()}, remote=${state.payload.state.head.debug()}`);
              tracing.info('Resetting storage to replace mismatched root');
              (() => {
                let root_1 = this._0.value.root.write();
                try {
                  root_1.value = null;
                } finally {
                  root_1.drop();
                }
              })()
              const _r3 = await this.hardReset().mapErr((e) => new MutationError('General', { _0: io.Error.other(e.toString()) }));
              if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
              _r3.drop();
            } finally {
              root.drop();
            }
          }
        }
        const collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID);
        try {
          const _r4 = await this._0.value.collectionset.get(collectionId);
          if (_r4.isErr()) return Result.Err(MutationError.fromRetrievalError(_r4.unwrapErr()));
          const storage = _r4.unwrap();
          try {
            const _r5 = await storage.deref().value.setState(state.clone());
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            _r5.drop();
            (() => {
              let root = this._0.value.root.write();
              try {
                _moved0 = true;
                root.value = state;
              } finally {
                root.drop();
              }
            })()
            const _t6 = this._0.value.systemReady.write();
            try {
              _t6.value = true;
            } finally {
              _t6.drop();
            }
            this._0.value.systemReadyNotify.notifyWaiters();
            return Result.Ok([]);
          } finally {
            storage.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        if (!_moved1) dropOwned(rootState);
      }
    } finally {
      if (!_moved0) state.drop();
    }
  }

  async hardReset(): Promise<Result<void, Error>> {
    const _r0 = await this._0.value.collectionset.deleteAllCollections();
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    (() => {
      let items = this._0.value.items.write();
      try {
        items.value.length = 0;
      } finally {
        items.drop();
      }
    })()
    (() => {
      let root = this._0.value.root.write();
      try {
        root.value = null;
      } finally {
        root.drop();
      }
    })()
    (() => {
      let collectionMap = this._0.value.collectionMap.write();
      try {
        collectionMap.value.clear();
      } finally {
        collectionMap.drop();
      }
    })()
    (() => {
      let systemReady = this._0.value.systemReady.write();
      try {
        systemReady.value = false;
      } finally {
        systemReady.drop();
      }
    })()
    this._0.value.reactor.systemReset();
    return Result.Ok([]);
  }

  isLoaded(): boolean {
    return this._0.value.loaded.get() != null;
  }

  async waitLoaded(): Promise<void> {
    if (!this.isLoaded()) {
      await this._0.value.loading.notified();
    }
  }

  async loadSystemCatalog(): Promise<Result<void, Error>> {
    if (this.isLoaded()) {
      return Result.Err(AnyhowError.msg('System catalog already loaded'));
    }
    const collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID);
    try {
      const _r0 = await this._0.value.collectionset.get(collectionId);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const storage = _r0.unwrap();
      try {
        let entities = [];
        let rootState = null;
        const retriever = LocalRetriever.new(storage.clone());
        try {
          const _t1 = new Selection(new Predicate('True', {}), null, null);
          try {
            const _r2 = await storage.deref().value.fetchStates(_t1);
            if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
            const _seq5 = _r2.unwrap();
            let _at6 = 0;
            try {
              while (_at6 < _seq5.length) {
                const state = _seq5[_at6++];
                let _moved3 = false;
                try {
                  const _r4 = await this._0.value.entities.withState(retriever, state.payload.entityId, collectionId.clone(), state.payload.state.clone());
                  if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
                  const [_entityChanged, entity] = _r4.unwrap();
                  const lwwBackend = entity.getBackend().expect('LWW Backend should exist');
                  try {
                    {
                      const _v1 = lwwBackend.value.get('item');
                      if (_v1 != null) {
                        const value = _v1;
                        const item = Item.fromValue(value).expect('Invalid sys item');
                        try {
                          {
                            const _v = item;
                            if (_v.is('SysRoot')) {
                              _moved3 = true;
                              rootState = state;
                            }
                          }
                          entities.push(entity);
                        } finally {
                          item.drop();
                        }
                      }
                    }
                  } finally {
                    lwwBackend.drop();
                  }
                } finally {
                  if (!_moved3) state.drop();
                }
              }
            } finally {
              dropOwned(_seq5.slice(_at6));
            }
          } finally {
            _t1.drop();
          }
          (() => {
            let items = this._0.value.items.write();
            try {
              items.value.push(...entities);
            } finally {
              items.drop();
            }
          })()
          const hasRoot = rootState != null;
          (() => {
            let root = this._0.value.root.write();
            try {
              root.value = rootState;
            } finally {
              root.drop();
            }
          })()
          if (hasRoot && this._0.value.durable) {
            const _t7 = this._0.value.systemReady.write();
            try {
              _t7.value = true;
            } finally {
              _t7.drop();
            }
            this._0.value.systemReadyNotify.notifyWaiters();
          }
          this._0.value.loaded.set([]).expect('Loading flag already set');
          this._0.value.loading.notifyWaiters();
          return Result.Ok([]);
        } finally {
          retriever.drop();
        }
      } finally {
        storage.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  clone(): SystemManager<SE, PA> {
    return new SystemManager(this._0.clone());
  }
}

class Inner<SE, PA> extends Struct {
  collectionset: CollectionSet<SE>;
  collectionMap: RwLock<HashMap<CollectionId, Entity>>;
  entities: WeakEntitySet;
  durable: boolean;
  root: RwLock<Attested<EntityState> | null>;
  items: RwLock<Entity[]>;
  loaded: OnceLock<void>;
  loading: Notify;
  systemReady: RwLock<boolean>;
  systemReadyNotify: Notify;
  reactor: Reactor<Entity, Attested<Event>>;

  constructor(collectionset: CollectionSet<SE>, collectionMap: RwLock<HashMap<CollectionId, Entity>>, entities: WeakEntitySet, durable: boolean, root: RwLock<Attested<EntityState> | null>, items: RwLock<Entity[]>, loaded: OnceLock<void>, loading: Notify, systemReady: RwLock<boolean>, systemReadyNotify: Notify, reactor: Reactor<Entity, Attested<Event>>) {
    super();
    this.collectionset = collectionset;
    this.collectionMap = collectionMap;
    this.entities = entities;
    this.durable = durable;
    this.root = root;
    this.items = items;
    this.loaded = loaded;
    this.loading = loading;
    this.systemReady = systemReady;
    this.systemReadyNotify = systemReadyNotify;
    this.reactor = reactor;
  }
}

export const SYSTEM_COLLECTION_ID: string = '_ankurah_system';

export const PROTECTED_COLLECTIONS: string[] = [SYSTEM_COLLECTION_ID];

export function Item_intoValue(self: Item): Result<Value | null, PropertyError> {
  const _r0 = serde_json.stringify((self).toJSON()).mapErr((_) => new PropertyError('InvalidValue', { value: '', ty: 'sys::Item' }));
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  return Result.Ok(new Value('String', { _0: _r0.unwrap() }));
}

export function Item_fromValue(value: Value | null): Result<Item, PropertyError> {
  {
    const _v = value;
    if (_v != null && (_v.is('String'))) {
      const { _0: string } = _v.value;
      const _r0 = serde_json.parse(string).mapErr((_) => new PropertyError('InvalidValue', { value: '', ty: 'sys::Item' }));
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      let _moved1 = false;
      const item = _r0.unwrap();
      try {
        _moved1 = true;
        return Result.Ok(item);
      } finally {
        if (!_moved1) item.drop();
      }
    } else {
    return Result.Err(new PropertyError('InvalidValue', { value: '', ty: 'sys::Item' }));
  }
  }
}

