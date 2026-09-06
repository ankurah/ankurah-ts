// MIRRORS: ankurah/storage/indexeddb-wasm/src/engine.rs
import { Struct, Result, Arc, tokio } from '@ankurah/base';
import { MutationError, RetrievalError, StorageCollection, StorageEngine } from '@ankurah/core';
import { IndexedDBBucket } from './collection';
import { Database } from './database';
import { cbFuture } from './util/cb_future';
import { Result_JsValue_require } from './util/require';
import { CollectionId } from '@ankurah/proto';

export class IndexedDBStorageEngine extends Struct implements StorageEngine {
  readonly db: Database;
  readonly prefixGuardDisabled: Arc<boolean>;

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
    this.prefixGuardDisabled.value = disabled;
  }

  async collection(collectionId: CollectionId): Promise<Result<Arc<StorageCollection>, RetrievalError>> {
    return Result.Ok(Arc.new(new IndexedDBBucket(this.db.clone(), collectionId.clone(), tokio.sync.Mutex.new([]), 0, this.prefixGuardDisabled.clone())));
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
      const _t3 = await cbFuture(entitiesRequest, 'success', 'error');
      try {
        const _r4 = _t3.require('await entities clear');
        if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
        _r4.drop();
      } finally {
        _t3.drop();
      }
      const _t5 = await cbFuture(entitiesTransaction, 'complete', 'error');
      try {
        const _r6 = _t5.require('complete entities transaction');
        if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
        _r6.drop();
      } finally {
        _t5.drop();
      }
      const _r7 = Result_JsValue_require(dbConnection.transactionWithStrAndMode('events', webSys.IdbTransactionMode.Readwrite), 'create events transaction');
      if (_r7.isErr()) return Result.Err(MutationError.fromAnyhowError(_r7.unwrapErr()));
      const eventsTransaction = _r7.unwrap();
      const _r8 = Result_JsValue_require(eventsTransaction.objectStore('events'), 'get events store');
      if (_r8.isErr()) return Result.Err(MutationError.fromAnyhowError(_r8.unwrapErr()));
      const eventsStore = _r8.unwrap();
      const _r9 = Result_JsValue_require(eventsStore.clear(), 'clear events store');
      if (_r9.isErr()) return Result.Err(MutationError.fromAnyhowError(_r9.unwrapErr()));
      const eventsRequest = _r9.unwrap();
      const _t10 = await cbFuture(eventsRequest, 'success', 'error');
      try {
        const _r11 = _t10.require('await events clear');
        if (_r11.isErr()) return Result.Err(_r11.unwrapErr());
        _r11.drop();
      } finally {
        _t10.drop();
      }
      const _t12 = await cbFuture(eventsTransaction, 'complete', 'error');
      try {
        const _r13 = _t12.require('complete events transaction');
        if (_r13.isErr()) return Result.Err(_r13.unwrapErr());
        _r13.drop();
      } finally {
        _t12.drop();
      }
      return Result.Ok(true);
    })());
  }
}

