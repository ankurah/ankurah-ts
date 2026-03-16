// MIRRORS: ankurah/storage/sqlite/src/engine.rs
//
// SQLite storage engine implementation.
//
// Platform-specific split (Exception E16):
//   Rust uses rusqlite (C bindings) + bb8 pool directly.
//   TS defines an abstract SqliteStorageEngine that takes a platform-specific
//   SQLite driver interface. Concrete implementations live in:
//     - @ankurah/storage-better-sqlite3 (Node.js)
//     - @ankurah/storage-expo-sqlite (React Native)

import type { CollectionId, EntityId, EventId, Attested, EntityState, Event } from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';
import type { StorageEngine, StorageCollection } from '@ankurah/core';

import { SqliteError } from './error.ts';

// ── SqliteDriver interface ────────────────────────────────────────────
// Divergence: Rust uses rusqlite directly. TS abstracts over platform-specific
// SQLite drivers via this interface [E16].

/**
 * Platform-specific SQLite driver interface.
 * Implemented by @ankurah/storage-better-sqlite3 and @ankurah/storage-expo-sqlite.
 */
export interface SqliteDriver {
  /** Execute a SQL statement that returns no rows. */
  execute(sql: string, params?: unknown[]): Promise<void>;
  /** Execute a SQL query and return all rows. */
  query<T = Record<string, unknown>>(sql: string, params?: unknown[]): Promise<T[]>;
  /** Execute a SQL query and return the first row, or null. */
  queryOne<T = Record<string, unknown>>(sql: string, params?: unknown[]): Promise<T | null>;
  /** Close the connection. */
  close(): Promise<void>;
}

// ── Constants ─────────────────────────────────────────────────────────

/** Default connection pool size. Rust: `pub const DEFAULT_POOL_SIZE: u32 = 10;` */
export const DEFAULT_POOL_SIZE = 10;

// ── SqliteStorageEngine ───────────────────────────────────────────────

/**
 * SQLite storage engine.
 *
 * Rust: `pub struct SqliteStorageEngine { pool: bb8::Pool<SqliteConnectionManager> }`
 * Divergence: Takes a SqliteDriver interface instead of a connection pool [E16].
 */
export class SqliteStorageEngine implements StorageEngine {
  private readonly driver: SqliteDriver;

  constructor(driver: SqliteDriver) {
    this.driver = driver;
  }

  /** Check if a collection name is safe for use as a table name. */
  static saneName(collection: string): boolean {
    if (collection.length === 0) return false;
    // Must start with letter or underscore
    if (!/^[a-zA-Z_]/.test(collection)) return false;
    // Must contain only alphanumeric, underscore, hyphen
    return /^[a-zA-Z_][a-zA-Z0-9_-]*$/.test(collection);
  }

  // ── StorageEngine implementation ──────────────────────────────────

  async collection(collectionId: CollectionId): Promise<StorageCollection> {
    // TODO: Implement — create tables if needed, return SqliteBucket
    throw new Error('TODO: SqliteStorageEngine.collection()');
  }

  async deleteAllCollections(): Promise<boolean> {
    // TODO: Implement — drop all collection tables
    throw new Error('TODO: SqliteStorageEngine.deleteAllCollections()');
  }
}

// ── SqliteBucket ──────────────────────────────────────────────────────

/**
 * A storage collection backed by SQLite tables.
 *
 * Rust: `pub struct SqliteBucket { pool, collection_id, state_table_name, event_table_name, columns }`
 */
export class SqliteBucket implements StorageCollection {
  private readonly driver: SqliteDriver;
  readonly collectionId: CollectionId;
  private readonly stateTableName: string;
  private readonly eventTableName: string;

  constructor(driver: SqliteDriver, collectionId: CollectionId) {
    this.driver = driver;
    this.collectionId = collectionId;
    this.stateTableName = `state_${collectionId}`;
    this.eventTableName = `event_${collectionId}`;
  }

  // ── StorageCollection implementation ──────────────────────────────

  async setState(state: Attested<EntityState>): Promise<boolean> {
    // TODO: Implement — UPSERT into state table
    throw new Error('TODO: SqliteBucket.setState()');
  }

  async getState(id: EntityId): Promise<Attested<EntityState>> {
    // TODO: Implement — SELECT from state table
    throw new Error('TODO: SqliteBucket.getState()');
  }

  async fetchStates(selection: Selection): Promise<Attested<EntityState>[]> {
    // TODO: Implement — SELECT with WHERE clause from sql_builder
    throw new Error('TODO: SqliteBucket.fetchStates()');
  }

  async addEvent(event: Attested<Event>): Promise<boolean> {
    // TODO: Implement — INSERT into event table
    throw new Error('TODO: SqliteBucket.addEvent()');
  }

  async getEvents(eventIds: EventId[]): Promise<Attested<Event>[]> {
    // TODO: Implement — SELECT from event table
    throw new Error('TODO: SqliteBucket.getEvents()');
  }

  async dumpEntityEvents(entityId: EntityId): Promise<Attested<Event>[]> {
    // TODO: Implement — SELECT all events for entity
    throw new Error('TODO: SqliteBucket.dumpEntityEvents()');
  }
}
