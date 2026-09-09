// MIRRORS: ankurah/storage/indexeddb-wasm/src/engine.rs
import { Struct, Result, Arc, dropOwned, unsupported, tokio } from '@ankurah/base';
import { MutationError, RetrievalError, StorageCollection, StorageEngine } from '@ankurah/core';
import { IndexedDBBucket } from './collection';
import { Database } from './database';
import { cbFuture } from './util/cb_future';
import { Result_Event_require, Result_JsValue_require } from './util/require';
import { CollectionId } from '@ankurah/proto';

export class IndexedDBStorageEngine extends Struct implements StorageEngine {
  readonly db: Database;
  prefixGuardDisabled: Arc<boolean>;

  constructor(db: Database, prefixGuardDisabled: Arc<boolean>) {
    super();
    this.db = db;
    this.prefixGuardDisabled = prefixGuardDisabled;
  }

  static async open(name: string): Promise<Result<IndexedDBStorageEngine, Error>> {
    const _r0 = await Database.open(name);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const db = _r0.unwrap();
    try {
      const _b2 = Arc.new(false);
      _moved1 = true;
      return Result.Ok(new IndexedDBStorageEngine(db, _b2));
    } finally {
      if (!_moved1) db.drop();
    }
  }

  static async cleanup(name: string): Promise<Result<void, Error>> {
    return await Database.cleanup(name);
  }

  name(): string {
    return this.db.name();
  }

  setPrefixGuardDisabled(disabled: boolean): void {
    unsupported('`store` WRITES what the `Arc<AtomicBool>` holds, and it is reached through an accessor that hands out the value rather than the place');
  }

  async collection(collectionId: CollectionId): Promise<Result<Arc<StorageCollection>, RetrievalError>> {
    let _moved1 = false;
    const _b0 = this.db.clone();
    try {
      let _moved3 = false;
      const _b2 = collectionId.clone();
      try {
        let _moved5 = false;
        const _b4 = tokio.sync.Mutex.new([]);
        try {
          const _b6 = this.prefixGuardDisabled.clone();
          _moved1 = true;
          _moved3 = true;
          _moved5 = true;
          return Result.Ok(Arc.new(new IndexedDBBucket(_b0, _b2, _b4, 0, _b6)));
        } finally {
          if (!_moved5) dropOwned(_b4);
        }
      } finally {
        if (!_moved3) dropOwned(_b2);
      }
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  }

  async deleteAllCollections(): Promise<Result<boolean, MutationError>> {
    const dbConnection = await this.db.getConnection();
    return await SendWrapper.new((async () => {
      const _r0 = Result_JsValue_require(dbConnection.transactionWithStrAndMode('entities', webSys.IdbTransactionMode.Readwrite), 'create entities transaction');
      if (_r0.isErr()) return Result.Err(MutationError.fromAnyhowError(_r0.unwrapErr()));
      const entitiesTransaction = _r0.unwrap();
      const _r1 = Result_JsValue_require(entitiesTransaction.objectStore('entities'), 'get entities store');
      if (_r1.isErr()) return Result.Err(MutationError.fromAnyhowError(_r1.unwrapErr()));
      const entitiesStore = _r1.unwrap();
      const _r2 = Result_JsValue_require(entitiesStore.clear(), 'clear entities store');
      if (_r2.isErr()) return Result.Err(MutationError.fromAnyhowError(_r2.unwrapErr()));
      const entitiesRequest = _r2.unwrap();
      const _r3 = Result_Event_require((await cbFuture(entitiesRequest, 'success', 'error')), 'await entities clear');
      if (_r3.isErr()) return Result.Err(MutationError.fromAnyhowError(_r3.unwrapErr()));
      _r3.drop();
      const _r4 = Result_Event_require((await cbFuture(entitiesTransaction, 'complete', 'error')), 'complete entities transaction');
      if (_r4.isErr()) return Result.Err(MutationError.fromAnyhowError(_r4.unwrapErr()));
      _r4.drop();
      const _r5 = Result_JsValue_require(dbConnection.transactionWithStrAndMode('events', webSys.IdbTransactionMode.Readwrite), 'create events transaction');
      if (_r5.isErr()) return Result.Err(MutationError.fromAnyhowError(_r5.unwrapErr()));
      const eventsTransaction = _r5.unwrap();
      const _r6 = Result_JsValue_require(eventsTransaction.objectStore('events'), 'get events store');
      if (_r6.isErr()) return Result.Err(MutationError.fromAnyhowError(_r6.unwrapErr()));
      const eventsStore = _r6.unwrap();
      const _r7 = Result_JsValue_require(eventsStore.clear(), 'clear events store');
      if (_r7.isErr()) return Result.Err(MutationError.fromAnyhowError(_r7.unwrapErr()));
      const eventsRequest = _r7.unwrap();
      const _r8 = Result_Event_require((await cbFuture(eventsRequest, 'success', 'error')), 'await events clear');
      if (_r8.isErr()) return Result.Err(MutationError.fromAnyhowError(_r8.unwrapErr()));
      _r8.drop();
      const _r9 = Result_Event_require((await cbFuture(eventsTransaction, 'complete', 'error')), 'complete events transaction');
      if (_r9.isErr()) return Result.Err(MutationError.fromAnyhowError(_r9.unwrapErr()));
      _r9.drop();
      return Result.Ok(true);
    })());
  }
}

