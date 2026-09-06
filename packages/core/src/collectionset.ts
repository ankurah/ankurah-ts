// MIRRORS: ankurah/core/src/collectionset.rs
import { Struct, Result, Arc, RwLock, dropOwned, HashMap, AsyncRwLock } from '@ankurah/base';
import { CollectionId } from '@ankurah/proto';
import { MutationError, RetrievalError } from './error';
import { StorageCollectionWrapper } from './storage';

export class CollectionSet<SE extends StorageEngine> extends Struct {
  _0: Arc<Inner<SE>>;

  constructor(_0: Arc<Inner<SE>>) {
    super();
    this._0 = _0;
  }

  static new<SE>(storageEngine: Arc<SE>): CollectionSet<SE> {
    return new CollectionSet(Arc.new(new Inner(storageEngine, new RwLock(new HashMap()))));
  }

  async get(id: CollectionId): Promise<Result<StorageCollectionWrapper, RetrievalError>> {
    let _moved0 = false;
    const collections = await this._0.value.collections.read();
    try {
      {
        const _v = collections.value.get(id);
        if (_v != null) {
          const store = _v;
          return Result.Ok(store.clone());
        }
      }
      _moved0 = true;
      collections.drop();
      const _r1 = await this._0.value.storageEngine.value.collection(id);
      if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
      let _moved2 = false;
      const collection = StorageCollectionWrapper.new(_r1.unwrap());
      try {
        let _moved3 = false;
        let collections_1 = await this._0.value.collections.write();
        try {
          {
            const _v1 = collections_1.value.entry(id.clone());
            if (_v1.is('Vacant')) {
              const { _0: entry } = _v1.value;
              entry.insert(collection.clone());
            } else {
            dropOwned(_v1);
          }
          }
          _moved3 = true;
          collections_1.drop();
          _moved2 = true;
          return Result.Ok(collection);
        } finally {
          if (!_moved3) collections_1.drop();
        }
      } finally {
        if (!_moved2) collection.drop();
      }
    } finally {
      if (!_moved0) collections.drop();
    }
  }

  async listCollections(): Promise<Result<CollectionId[], RetrievalError>> {
    const memoryCollections = await this._0.value.collections.read();
    try {
      return Result.Ok([...memoryCollections.value.keys()]);
    } finally {
      memoryCollections.drop();
    }
  }

  async deleteAllCollections(): Promise<Result<boolean, MutationError>> {
    await (async () => {
      let collections = await this._0.value.collections.write();
      try {
        collections.value.clear();
      } finally {
        collections.drop();
      }
    })();
    return await this._0.value.storageEngine.value.deleteAllCollections();
  }

  clone(): CollectionSet<SE> {
    return new CollectionSet(this._0.clone());
  }
}

export class Inner<SE> extends Struct {
  storageEngine: Arc<SE>;
  collections: AsyncRwLock<HashMap<CollectionId, StorageCollectionWrapper>>;

  constructor(storageEngine: Arc<SE>, collections: AsyncRwLock<HashMap<CollectionId, StorageCollectionWrapper>>) {
    super();
    this.storageEngine = storageEngine;
    this.collections = collections;
  }
}

