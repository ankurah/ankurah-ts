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

import { Selection } from '@ankurah/ankql';
import {
  CollectionId,
  EntityId,
  EventId,
  Attested,
  AttestationSet,
  EntityState,
  Event,
  State,
  StateBuffers,
  OperationSet,
  Clock,
  BincodeWriter,
  BincodeReader,
} from '@ankurah/proto';
import type { StorageEngine, StorageCollection } from '@ankurah/core';
import { backendFromString, evaluatePredicate, type Filterable, type Value } from '@ankurah/core';
import { RetrievalError, MutationError } from '@ankurah/core';

import { SqliteError } from './error.ts';
import { SqlBuilder, splitPredicateForSqlite } from './sql_builder.ts';
import {
  type SqliteValue,
  sqliteValueType,
  sqliteValueIsJsonb,
  sqliteValueToParam,
  sqliteValueFromValue,
} from './value.ts';

// ── SqliteDriver interface ────────────────────────────────────────────
// Divergence: Rust uses rusqlite directly. TS abstracts over platform-specific
// SQLite drivers via this interface [E16].

/**
 * Platform-specific SQLite driver interface.
 * Implemented by @ankurah/storage-better-sqlite3 and @ankurah/storage-expo-sqlite.
 */
export interface SqliteDriver {
  /** Execute a SQL statement that returns no rows. Returns number of rows changed. */
  execute(sql: string, params?: unknown[]): number;
  /** Execute a SQL query and return all rows. */
  query<T = Record<string, unknown>>(sql: string, params?: unknown[]): T[];
  /** Execute a SQL query and return the first row, or null. */
  queryOne<T = Record<string, unknown>>(sql: string, params?: unknown[]): T | null;
  /** Close the connection. */
  close(): void;
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
  // Divergence: Rust uses bb8::Pool<SqliteConnectionManager>; TS uses SqliteDriver [E16]
  private readonly driver: SqliteDriver;

  constructor(driver: SqliteDriver) {
    this.driver = driver;
  }

  /** Check if a collection name is safe for use as a table name. */
  static saneName(collection: string): boolean {
    for (const char of collection) {
      if (/[a-zA-Z0-9]/.test(char)) continue;
      if (char === '_' || char === '.' || char === ':') continue;
      return false;
    }
    return true;
  }

  /** Get a reference to the driver (for testing/diagnostics). */
  getDriver(): SqliteDriver {
    return this.driver;
  }

  // ── StorageEngine implementation ──────────────────────────────────

  async collection(collectionId: CollectionId): Promise<StorageCollection> {
    if (!SqliteStorageEngine.saneName(collectionId.value)) {
      throw RetrievalError.invalidBucketName();
    }

    const bucket = new SqliteBucket(this.driver, collectionId);

    // Create tables if they don't exist
    createStateTable(this.driver, collectionId);
    createEventTable(this.driver, collectionId);

    // Rebuild column cache
    bucket.rebuildColumnsCache();

    return bucket;
  }

  async deleteAllCollections(): Promise<boolean> {
    // Get all table names
    const tables = this.driver.query<{ name: string }>(
      "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    );

    if (tables.length === 0) {
      return false;
    }

    for (const table of tables) {
      this.driver.execute(`DROP TABLE IF EXISTS "${table.name}"`);
    }

    return true;
  }
}

// ── Table creation helpers ────────────────────────────────────────────

function createStateTable(driver: SqliteDriver, collectionId: CollectionId): void {
  const tableName = collectionId.value;
  const query = `CREATE TABLE IF NOT EXISTS "${tableName}"(
            "id" TEXT PRIMARY KEY,
            "state_buffer" BLOB NOT NULL,
            "head" TEXT NOT NULL,
            "attestations" BLOB
        )`;
  driver.execute(query);
}

function createEventTable(driver: SqliteDriver, collectionId: CollectionId): void {
  const tableName = `${collectionId.value}_event`;
  const query = `CREATE TABLE IF NOT EXISTS "${tableName}"(
            "id" TEXT PRIMARY KEY,
            "entity_id" TEXT,
            "operations" BLOB,
            "parent" TEXT,
            "attestations" BLOB
        )`;
  driver.execute(query);

  // Create index on entity_id for efficient dump_entity_events queries
  const indexQuery = `CREATE INDEX IF NOT EXISTS "${tableName}_entity_id_idx" ON "${tableName}"("entity_id")`;
  driver.execute(indexQuery);
}

// ── SqliteColumn ──────────────────────────────────────────────────────

/** Column metadata */
interface SqliteColumn {
  name: string;
  dataType: string;
}

// ── SqliteBucket ──────────────────────────────────────────────────────

/**
 * A storage collection backed by SQLite tables.
 *
 * Rust: `pub struct SqliteBucket { pool, collection_id, state_table_name, event_table_name, columns, ddl_lock }`
 * Divergence: Uses SqliteDriver instead of bb8 pool [E16].
 * Divergence: No ddl_lock — JS is single-threaded [E8].
 */
export class SqliteBucket implements StorageCollection {
  private readonly driver: SqliteDriver;
  readonly collectionId: CollectionId;
  private readonly stateTableName: string;
  private readonly eventTableName: string;
  private columns: SqliteColumn[];

  constructor(driver: SqliteDriver, collectionId: CollectionId) {
    this.driver = driver;
    this.collectionId = collectionId;
    this.stateTableName = collectionId.value;
    this.eventTableName = `${collectionId.value}_event`;
    this.columns = [];
  }

  private stateTable(): string {
    return this.stateTableName;
  }

  private eventTable(): string {
    return this.eventTableName;
  }

  /** Returns all column names currently in the schema cache. */
  existingColumns(): string[] {
    return this.columns.map((c) => c.name);
  }

  /** Check if a column exists in the schema cache. */
  hasColumn(name: string): boolean {
    return this.columns.some((c) => c.name === name);
  }

  rebuildColumnsCache(): void {
    const tableName = this.stateTable();
    const rows = this.driver.query<{ name: string; type: string }>(
      `PRAGMA table_info("${tableName}")`,
    );
    this.columns = rows.map((row) => ({ name: row.name, dataType: row.type }));
  }

  private addMissingColumns(missing: Array<[string, string]>): void {
    if (missing.length === 0) return;

    // Divergence: No DDL lock — JS is single-threaded [E8]
    this.rebuildColumnsCache();

    const tableName = this.stateTable();
    for (const [column, datatype] of missing) {
      if (SqliteStorageEngine.saneName(column) && !this.hasColumn(column)) {
        const alterQuery = `ALTER TABLE "${tableName}" ADD COLUMN "${column}" ${datatype}`;
        this.driver.execute(alterQuery);
      }
    }

    this.rebuildColumnsCache();
  }

  // ── StorageCollection implementation ──────────────────────────────

  async setState(state: Attested<EntityState>): Promise<boolean> {
    // Ensure head is not empty for new records
    if (state.payload.state.head.isEmpty()) {
      console.warn(`Warning: Empty head detected for entity ${state.payload.entityId}`);
    }

    // Serialize state_buffers via bincode
    const stateBufferWriter = new BincodeWriter();
    state.payload.state.stateBuffers.encode(stateBufferWriter);
    const stateBufferBlob = stateBufferWriter.finish();

    // Serialize head as JSON
    const headJson = JSON.stringify(state.payload.state.head.toStrings());

    // Serialize attestations via bincode
    const attestationsWriter = new BincodeWriter();
    state.attestations.encode(attestationsWriter);
    const attestationsBlob = attestationsWriter.finish();

    const id = state.payload.entityId.toBase64();

    // Collect materialized columns (with JSONB flag for proper SQL generation)
    const materialized: Array<{ name: string; value: SqliteValue | null; isJsonb: boolean }> = [];
    const seenProperties = new Set<string>();

    for (const [name, stateBuffer] of state.payload.state.stateBuffers) {
      const backend = backendFromString(name, stateBuffer);
      for (const [column, value] of backend.propertyValues()) {
        if (seenProperties.has(column)) continue;
        seenProperties.add(column);

        const sqliteValue: SqliteValue | null = value !== null ? sqliteValueFromValue(value) : null;
        const isJsonb = sqliteValue !== null && sqliteValueIsJsonb(sqliteValue);

        if (!this.hasColumn(column)) {
          if (sqliteValue !== null) {
            this.addMissingColumns([[column, sqliteValueType(sqliteValue)]]);
          } else {
            continue;
          }
        }

        materialized.push({ name: column, value: sqliteValue, isJsonb });
      }
    }

    // Build the UPSERT query
    const BASE_COLUMNS = ['id', 'state_buffer', 'head', 'attestations'] as const;

    const tableName = this.stateTable();
    const columns: string[] = [...BASE_COLUMNS];
    const values: unknown[] = [id, stateBufferBlob, headJson, attestationsBlob];

    // Track which placeholders need jsonb() wrapper (base columns don't)
    const placeholderIsJsonb: boolean[] = [false, false, false, false];

    for (const mat of materialized) {
      columns.push(mat.name);
      values.push(mat.value !== null ? sqliteValueToParam(mat.value) : null);
      placeholderIsJsonb.push(mat.isJsonb);
    }

    const columnsStr = columns.map((c) => `"${c}"`).join(', ');
    // Use jsonb(?) for JSONB columns to convert JSON text to JSONB binary format
    const placeholders = placeholderIsJsonb
      .map((isJsonb) => (isJsonb ? 'jsonb(?)' : '?'))
      .join(', ');
    const updateStr = columns
      .slice(1)
      .map((c) => `"${c}" = excluded."${c}"`)
      .join(', ');

    // First, get the old head if the entity exists
    const oldRow = this.driver.queryOne<{ head: string }>(
      `SELECT "head" FROM "${tableName}" WHERE "id" = ?`,
      [id],
    );

    // Execute the UPSERT
    const query = `INSERT INTO "${tableName}"(${columnsStr}) VALUES(${placeholders})
               ON CONFLICT("id") DO UPDATE SET ${updateStr}`;
    this.driver.execute(query, values);

    // Determine if state changed
    let changed: boolean;
    if (oldRow !== null) {
      const oldHead = Clock.fromStrings(JSON.parse(oldRow.head));
      changed = !oldHead.equals(state.payload.state.head);
    } else {
      changed = true;
    }

    return changed;
  }

  async getState(id: EntityId): Promise<Attested<EntityState>> {
    const tableName = this.stateTable();
    const idStr = id.toBase64();

    type StateRow = {
      id: string;
      state_buffer: Uint8Array;
      head: string;
      attestations: Uint8Array;
    };

    const row = this.driver.queryOne<StateRow>(
      `SELECT "id", "state_buffer", "head", "attestations" FROM "${tableName}" WHERE "id" = ?`,
      [idStr],
    );

    if (row === null) {
      throw RetrievalError.entityNotFound(id);
    }

    // Deserialize state_buffers
    const stateBufferBytes = row.state_buffer instanceof Uint8Array
      ? row.state_buffer
      : new Uint8Array(row.state_buffer as ArrayBuffer);
    const stateBuffers = StateBuffers.decode(new BincodeReader(stateBufferBytes));

    // Deserialize head
    const head = Clock.fromStrings(JSON.parse(row.head));

    // Deserialize attestations
    const attestationsBytes = row.attestations instanceof Uint8Array
      ? row.attestations
      : new Uint8Array(row.attestations as ArrayBuffer);
    const attestations = AttestationSet.decode(new BincodeReader(attestationsBytes));

    return new Attested(
      new EntityState(id, this.collectionId, new State(stateBuffers, head)),
      attestations,
    );
  }

  async fetchStates(selection: Selection): Promise<Attested<EntityState>[]> {
    // Pre-filter selection based on cached schema to avoid undefined column errors.
    const referenced = selection.referencedColumns();
    const cached = this.existingColumns();
    const unknownToCache = referenced.filter((col: string) => !cached.includes(col));

    // Refresh cache if we see columns we haven't seen before
    if (unknownToCache.length > 0) {
      this.rebuildColumnsCache();
    }

    // Now check with (possibly refreshed) cache - columns still missing truly don't exist
    const existing = this.existingColumns();
    const missing: string[] = referenced.filter((col: string) => !existing.includes(col));

    const effectiveSelection = missing.length === 0
      ? selection
      : selection.assumeNull(missing);

    // Split predicate for pushdown
    const split = splitPredicateForSqlite(effectiveSelection.predicate);
    const needsPostFilter = split.needsPostFilter();
    const remainingPredicate = split.remainingPredicate;

    // Build SQL
    const sqlSelection = new Selection(
      split.sqlPredicate,
      effectiveSelection.orderBy,
      needsPostFilter ? null : effectiveSelection.limit,
    );

    const builder = SqlBuilder.withFields(['id', 'state_buffer', 'head', 'attestations']);
    builder.setTableName(this.stateTable());
    builder.selection(sqlSelection);

    const [sql, params] = builder.build();

    type StateRow = {
      id: string;
      state_buffer: Uint8Array;
      head: string;
      attestations: Uint8Array;
    };

    const rows = this.driver.query<StateRow>(sql, params);

    let results: Attested<EntityState>[] = [];
    for (const row of rows) {
      const entityId = EntityId.fromBase64(row.id);

      const stateBufferBytes = row.state_buffer instanceof Uint8Array
        ? row.state_buffer
        : new Uint8Array(row.state_buffer as ArrayBuffer);
      const stateBuffers = StateBuffers.decode(new BincodeReader(stateBufferBytes));

      const head = Clock.fromStrings(JSON.parse(row.head));

      const attestationsBytes = row.attestations instanceof Uint8Array
        ? row.attestations
        : new Uint8Array(row.attestations as ArrayBuffer);
      const attestations = AttestationSet.decode(new BincodeReader(attestationsBytes));

      results.push(
        new Attested(
          new EntityState(entityId, this.collectionId, new State(stateBuffers, head)),
          attestations,
        ),
      );
    }

    // Post-filter if needed
    if (needsPostFilter) {
      results = postFilterStates(results, remainingPredicate, this.collectionId);

      if (effectiveSelection.limit !== null) {
        results = results.slice(0, effectiveSelection.limit);
      }
    }

    return results;
  }

  async addEvent(entityEvent: Attested<Event>): Promise<boolean> {
    // Serialize operations via bincode
    const opsWriter = new BincodeWriter();
    entityEvent.payload.operations.encode(opsWriter);
    const operationsBlob = opsWriter.finish();

    // Serialize attestations via bincode
    const attestationsWriter = new BincodeWriter();
    entityEvent.attestations.encode(attestationsWriter);
    const attestationsBlob = attestationsWriter.finish();

    // Serialize parent as JSON
    const parentJson = JSON.stringify(entityEvent.payload.parent.toStrings());

    const tableName = this.eventTable();
    const eventId = entityEvent.payload.id().toBase64();
    const entityId = entityEvent.payload.entityId.toBase64();

    const query = `INSERT INTO "${tableName}"("id", "entity_id", "operations", "parent", "attestations") VALUES(?, ?, ?, ?, ?)
               ON CONFLICT ("id") DO NOTHING`;

    const affected = this.driver.execute(query, [eventId, entityId, operationsBlob, parentJson, attestationsBlob]);
    return affected > 0;
  }

  async getEvents(eventIds: EventId[]): Promise<Attested<Event>[]> {
    if (eventIds.length === 0) {
      return [];
    }

    const tableName = this.eventTable();
    const idStrings = eventIds.map((id) => id.toBase64());

    const placeholders = idStrings.map(() => '?').join(', ');
    const query = `SELECT "id", "entity_id", "operations", "parent", "attestations" FROM "${tableName}" WHERE "id" IN (${placeholders})`;

    type EventRow = {
      id: string;
      entity_id: string;
      operations: Uint8Array;
      parent: string;
      attestations: Uint8Array;
    };

    const rows = this.driver.query<EventRow>(query, idStrings);

    const events: Attested<Event>[] = [];
    for (const row of rows) {
      const entityId = EntityId.fromBase64(row.entity_id);

      const opsBytes = row.operations instanceof Uint8Array
        ? row.operations
        : new Uint8Array(row.operations as ArrayBuffer);
      const operations = OperationSet.decode(new BincodeReader(opsBytes));

      const parent = Clock.fromStrings(JSON.parse(row.parent));

      const attestationsBytes = row.attestations instanceof Uint8Array
        ? row.attestations
        : new Uint8Array(row.attestations as ArrayBuffer);
      const attestations = AttestationSet.decode(new BincodeReader(attestationsBytes));

      events.push(
        new Attested(
          new Event(this.collectionId, entityId, operations, parent),
          attestations,
        ),
      );
    }

    return events;
  }

  async dumpEntityEvents(entityId: EntityId): Promise<Attested<Event>[]> {
    const tableName = this.eventTable();
    const entityIdStr = entityId.toBase64();

    type EventRow = {
      id: string;
      operations: Uint8Array;
      parent: string;
      attestations: Uint8Array;
    };

    const rows = this.driver.query<EventRow>(
      `SELECT "id", "operations", "parent", "attestations" FROM "${tableName}" WHERE "entity_id" = ?`,
      [entityIdStr],
    );

    const events: Attested<Event>[] = [];
    for (const row of rows) {
      const opsBytes = row.operations instanceof Uint8Array
        ? row.operations
        : new Uint8Array(row.operations as ArrayBuffer);
      const operations = OperationSet.decode(new BincodeReader(opsBytes));

      const parent = Clock.fromStrings(JSON.parse(row.parent));

      const attestationsBytes = row.attestations instanceof Uint8Array
        ? row.attestations
        : new Uint8Array(row.attestations as ArrayBuffer);
      const attestations = AttestationSet.decode(new BincodeReader(attestationsBytes));

      events.push(
        new Attested(
          new Event(this.collectionId, entityId, operations, parent),
          attestations,
        ),
      );
    }

    return events;
  }
}

// ── Post-filter helper ───────────────────────────────────────────────

/**
 * Post-filter EntityStates using a predicate that couldn't be pushed to SQL.
 *
 * Rust: `fn post_filter_states`
 * Divergence: Rust uses TemporaryEntity; TS uses Filterable adapter (same approach as MemoryStorageCollection) [E16].
 */
function postFilterStates(
  states: Attested<EntityState>[],
  predicate: import('@ankurah/ankql').Predicate,
  collectionId: CollectionId,
): Attested<EntityState>[] {
  return states.filter((attested) => {
    try {
      const filterable = entityStateAsFilterable(attested.payload, collectionId);
      return evaluatePredicate(filterable, predicate);
    } catch (e) {
      console.warn(`Post-filter evaluation error for entity ${attested.payload.entityId}: ${e}`);
      return false;
    }
  });
}

/**
 * Creates a Filterable adapter from an EntityState.
 * Equivalent to Rust's TemporaryEntity -- reconstitutes property backends
 * from state buffers to enable field-level value access for predicate evaluation.
 */
function entityStateAsFilterable(
  entityState: EntityState,
  collectionId: CollectionId,
): Filterable {
  let backends: Map<string, import('@ankurah/core').PropertyBackend> | null = null;

  function getBackends(): Map<string, import('@ankurah/core').PropertyBackend> {
    if (backends === null) {
      backends = new Map();
      for (const [name, buffer] of entityState.state.stateBuffers) {
        backends.set(name, backendFromString(name, buffer));
      }
    }
    return backends;
  }

  return {
    collection(): string {
      return collectionId.value;
    },
    value(name: string): Value | null {
      if (name === 'id') {
        return { type: 'EntityId', value: entityState.entityId };
      }
      for (const backend of getBackends().values()) {
        const v = backend.propertyValue(name);
        if (v !== null) return v;
      }
      return null;
    },
  };
}
