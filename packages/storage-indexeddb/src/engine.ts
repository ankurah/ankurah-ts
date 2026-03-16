// MIRRORS: ankurah/storage/indexeddb-wasm/src/engine.rs

// Divergence: Rust uses SendWrapper for !Send JsValue and async_trait. [E16]
// In TS, we use native IndexedDB API directly.

import type { StorageEngine, StorageCollection } from '@ankurah/core';
import type { CollectionId } from '@ankurah/proto';
import { Database } from './database.ts';
import { IndexedDBBucket } from './collection.ts';
import { cbFuture } from './util/cb_future.ts';

export class IndexedDBStorageEngine implements StorageEngine {
  readonly db: Database;
  /** For testing: enable/disable prefix guard at runtime */
  prefixGuardDisabled: boolean = false;

  private constructor(db: Database) {
    this.db = db;
  }

  static async open(name: string): Promise<IndexedDBStorageEngine> {
    const db = await Database.open(name);
    return new IndexedDBStorageEngine(db);
  }

  static async cleanup(name: string): Promise<void> {
    await Database.cleanup(name);
  }

  /** Get the database name */
  name(): string {
    return this.db.name();
  }

  /** For tests: enable/disable prefix guard at runtime */
  setPrefixGuardDisabled(disabled: boolean): void {
    this.prefixGuardDisabled = disabled;
  }

  // StorageEngine interface

  async collection(collectionId: CollectionId): Promise<StorageCollection> {
    const bucket = new IndexedDBBucket(this.db, collectionId);
    bucket.prefixGuardDisabled = this.prefixGuardDisabled;
    return bucket;
  }

  async deleteAllCollections(): Promise<boolean> {
    const dbConnection = await this.db.getConnection();

    // Clear entities store
    const entitiesTx = dbConnection.transaction('entities', 'readwrite');
    const entitiesStore = entitiesTx.objectStore('entities');
    const entitiesClearReq = entitiesStore.clear();
    await cbFuture(entitiesClearReq, 'success', 'error');
    await cbFuture(entitiesTx, 'complete', 'error');

    // Clear events store
    const eventsTx = dbConnection.transaction('events', 'readwrite');
    const eventsStore = eventsTx.objectStore('events');
    const eventsClearReq = eventsStore.clear();
    await cbFuture(eventsClearReq, 'success', 'error');
    await cbFuture(eventsTx, 'complete', 'error');

    return true;
  }
}
