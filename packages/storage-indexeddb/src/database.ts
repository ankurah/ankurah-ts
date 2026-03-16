// MIRRORS: ankurah/storage/indexeddb-wasm/src/database.rs

// Divergence: Rust uses wasm-bindgen + web_sys types (IdbDatabase, IdbFactory, etc.) [E16]
// with SendWrapper for !Send safety and CBFuture/CBRace for async event bridging.
// In TS, we use the native IndexedDB API (window.indexedDB) directly with Promises.

import type { KeySpec } from '@ankurah/core';
import { keySpecNameWith, indexKeyPartFullPath } from '@ankurah/core';
import { NavigatorLock } from './util/navigator_lock.ts';
import { cbFuture } from './util/cb_future.ts';

/**
 * IndexedDB database connection manager with lazy reopen and index creation.
 *
 * Mirrors Rust `Database(Arc<Inner>)` which wraps a connection + index cache.
 */
export class Database {
  private connection: Connection;
  private readonly dbName: string;
  /** Cache of existing index names to avoid repeated checks */
  private readonly indexCache: Set<string> = new Set();
  /** Mutex for serializing connection access */
  private connectionLock: Promise<void> = Promise.resolve();

  private constructor(connection: Connection, dbName: string) {
    this.connection = connection;
    this.dbName = dbName;
  }

  static async open(dbName: string): Promise<Database> {
    const connection = await Connection.open(dbName);
    return new Database(connection, dbName);
  }

  /** Get the current database connection; lazily re-open if stale */
  async getConnection(): Promise<IDBDatabase> {
    if (this.connection.isStale()) {
      const reopened = await Connection.open(this.dbName);
      this.connection = reopened;
    }
    return this.connection.db;
  }

  /** Close the database connection */
  close(): void {
    this.connection.close();
  }

  /** Ensure an index exists, creating it if necessary via database version upgrade */
  async assureIndexExists(indexSpec: KeySpec): Promise<void> {
    const name = keySpecNameWith(indexSpec, '', '__');
    if (this.indexCache.has(name)) {
      return;
    }
    if (this.connection.hasIndex(name)) {
      this.indexCache.add(name);
      return;
    }

    const lockName = `ankurah-idb-upgrade-${this.dbName}`;
    const db = this;

    await NavigatorLock.with(lockName, async () => {
      if (db.indexCache.has(name)) {
        return;
      }
      if (db.connection.hasIndex(name)) {
        db.indexCache.add(name);
        return;
      }
      const currentVersion = db.connection.version();
      db.connection.close();
      const newConnection = await Connection.openWithIndex(db.dbName, currentVersion + 1, indexSpec);
      db.connection = newConnection;
      db.indexCache.add(name);
    });
  }

  /** Get database name */
  name(): string {
    return this.dbName;
  }

  /** Cleanup database (delete it entirely) */
  static async cleanup(dbName: string): Promise<void> {
    const factory = Database.getFactory();
    const request = factory.deleteDatabase(dbName);
    await cbFuture(request, ['success', 'blocked'], 'error');
  }

  /** Get the IDBFactory */
  private static getFactory(): IDBFactory {
    if (typeof indexedDB !== 'undefined') {
      return indexedDB;
    }
    throw new Error('IndexedDB not available');
  }
}

/**
 * Internal connection wrapper.
 *
 * Mirrors Rust `Connection` which wraps `SendWrapper<IdbDatabase>` with stale detection.
 */
class Connection {
  readonly db: IDBDatabase;
  private stale: boolean = false;
  private onversionchangeHandler: ((event: Event) => void) | null = null;

  private constructor(db: IDBDatabase) {
    this.db = db;

    // Set up versionchange handler for lazy reopen
    this.onversionchangeHandler = () => {
      this.stale = true;
      console.warn('Version change event received - closing database');
      db.close();
    };
    db.onversionchange = this.onversionchangeHandler;
  }

  isStale(): boolean {
    return this.stale;
  }

  version(): number {
    return this.db.version;
  }

  /** Check whether the `entities` store already has an index with the given name */
  hasIndex(indexName: string): boolean {
    try {
      const tx = this.db.transaction('entities', 'readonly');
      const store = tx.objectStore('entities');
      return store.indexNames.contains(indexName);
    } catch {
      return false;
    }
  }

  /** Close the database connection */
  close(): void {
    this.db.onversionchange = null;
    this.db.close();
  }

  /** Open or create a new database connection with default schema */
  static async open(dbName: string): Promise<Connection> {
    if (!dbName) {
      throw new Error('Database name cannot be empty');
    }

    return Connection.openNew(dbName, undefined, (db, _oldVersion) => {
      // Create object stores if they don't exist
      if (!db.objectStoreNames.contains('entities')) {
        const store = db.createObjectStore('entities');
        store.createIndex('__collection__id', ['__collection', 'id']);
      }
      if (!db.objectStoreNames.contains('events')) {
        const eventsStore = db.createObjectStore('events');
        eventsStore.createIndex('by_entity_id', '__entity_id');
      }
    });
  }

  /** Open database connection with a specific index to be created */
  static async openWithIndex(dbName: string, version: number, indexSpec: KeySpec): Promise<Connection> {
    const indexName = keySpecNameWith(indexSpec, '', '__');

    return Connection.openNew(dbName, version, (_db, _oldVersion, transaction) => {
      if (!transaction) return;
      const store = transaction.objectStore('entities');
      // Use full_path() to support JSON sub-paths (e.g., "context.session_id")
      const keyPath = indexSpec.keyparts.map(kp => indexKeyPartFullPath(kp));
      store.createIndex(indexName, keyPath);
    });
  }

  /** Internal: open database with optional version and upgrade callback */
  private static async openNew(
    dbName: string,
    version: number | undefined,
    onUpgradeNeeded: (db: IDBDatabase, oldVersion: number, transaction?: IDBTransaction) => void,
  ): Promise<Connection> {
    const factory = Database['getFactory']();
    const request = version !== undefined
      ? factory.open(dbName, version)
      : factory.open(dbName);

    request.onupgradeneeded = (event) => {
      const db = request.result;
      const oldVersion = event.oldVersion;
      onUpgradeNeeded(db, oldVersion, request.transaction ?? undefined);
    };

    await cbFuture(request, 'success', 'error');

    return new Connection(request.result);
  }
}
