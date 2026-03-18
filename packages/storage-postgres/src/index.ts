// MIRRORS: ankurah/storage/postgres/src/lib.rs

import type { StorageEngine, StorageCollection } from '@ankurah/core';
import { MutationError, RetrievalError, StateError } from '@ankurah/core';
import { backendFromString, evaluatePredicate } from '@ankurah/core';
import type { Filterable } from '@ankurah/core';
import {
  Attestation,
  Attested,
  AttestationSet,
  EntityState,
  EntityId,
  Event,
  State,
  StateBuffers,
  OperationSet,
  Clock,
  BincodeWriter,
  BincodeReader,
} from '@ankurah/proto';
import type { EventId, CollectionId } from '@ankurah/proto';
import { Selection, type Predicate } from '@ankurah/ankql';
import { SqlBuilder, splitPredicateForPostgres } from './sql_builder.ts';
import { type PGValue, pgValueFromValue, pgValuePostgresType } from './value.ts';

export { SqlBuilder, SqlGenerationError, SplitPredicate, splitPredicateForPostgres } from './sql_builder.ts';
export { type PGValue, pgValueFromValue, pgValuePostgresType } from './value.ts';

// ── Constants ────────────────────────────────────────────────────────

/// Default connection pool size for `Postgres.open()`.
/// Production applications should configure their own pool via `Postgres.new()`.
export const DEFAULT_POOL_SIZE = 15;

/// Default connection timeout in seconds
export const DEFAULT_CONNECTION_TIMEOUT_SECS = 30;

// ── PostgresClient interface ─────────────────────────────────────────
// Divergence: Rust uses bb8 + tokio-postgres with concrete types.
// TS defines an interface so callers can supply any PG client (pg, postgres.js, etc.) [E8].

export interface PostgresQueryResult {
  rows: Record<string, unknown>[];
  rowCount: number;
}

export interface PostgresClient {
  query(sql: string, params?: unknown[]): Promise<PostgresQueryResult>;
  queryOne(sql: string, params?: unknown[]): Promise<Record<string, unknown>>;
}

export interface PostgresPool {
  getClient(): Promise<PostgresClient>;
}

// ── Postgres ─────────────────────────────────────────────────────────

export class Postgres implements StorageEngine {
  private pool: PostgresPool;

  constructor(pool: PostgresPool) {
    this.pool = pool;
  }

  // Divergence: Rust `open(uri)` creates a bb8 pool. TS relies on caller to provide pool [E8].

  static saneName(collection: string): boolean {
    for (const char of collection) {
      if (/[a-zA-Z0-9]/.test(char)) continue;
      if (char === '_' || char === '.' || char === ':') continue;
      return false;
    }
    return true;
  }

  // impl StorageEngine for Postgres

  async collection(collectionId: CollectionId): Promise<StorageCollection> {
    if (!Postgres.saneName(collectionId.asStr())) {
      throw RetrievalError.invalidBucketName();
    }

    const client = await this.pool.getClient();

    // Get the current schema from the database
    const schemaRow = await client.queryOne('SELECT current_database()', []);
    const schema = schemaRow['current_database'] as string;

    const bucket = new PostgresBucket(
      this.pool,
      collectionId,
      schema,
    );

    // Acquire advisory lock to serialize DDL operations for this collection
    const lockKey = await acquireDdlLock(client, collectionId.asStr());

    try {
      // Create tables if they don't exist (protected by advisory lock)
      await bucket.createStateTable(client);
      await bucket.createEventTable(client);
      await bucket.rebuildColumnsCache(client);
    } finally {
      // Always release the lock, even if DDL failed
      await releaseDdlLock(client, lockKey);
    }

    return bucket;
  }

  async deleteAllCollections(): Promise<boolean> {
    const client = await this.pool.getClient();

    // Get all tables in the public schema
    const query = `
      SELECT table_name
      FROM information_schema.tables
      WHERE table_schema = 'public'
    `;

    const result = await client.query(query, []);
    if (result.rows.length === 0) {
      return false;
    }

    // Drop each table
    // Divergence: Rust wraps in a transaction. TS does sequential drops.
    // TODO: Add transaction support when PostgresClient interface supports it [E8].
    for (const row of result.rows) {
      const tableName = row['table_name'] as string;
      const dropQuery = `DROP TABLE IF EXISTS "${tableName}"`;
      await client.query(dropQuery, []);
    }

    return true;
  }
}

// ── Advisory locking ─────────────────────────────────────────────────

/// Compute advisory lock key from a string identifier
function advisoryLockKey(identifier: string): bigint {
  // Divergence: Rust uses DefaultHasher. TS uses simple FNV-1a hash [E8].
  let hash = 2166136261n;
  for (let i = 0; i < identifier.length; i++) {
    hash ^= BigInt(identifier.charCodeAt(i));
    hash = (hash * 16777619n) & 0xFFFFFFFFFFFFFFFFn;
  }
  // Convert to signed i64 range
  return hash > 0x7FFFFFFFFFFFFFFFn ? hash - 0x10000000000000000n : hash;
}

/// Acquire a PostgreSQL advisory lock for DDL operations on a collection
async function acquireDdlLock(client: PostgresClient, collectionId: string): Promise<bigint> {
  const lockKey = advisoryLockKey(`ankurah_ddl:${collectionId}`);
  try {
    await client.query('SELECT pg_advisory_lock($1)', [lockKey]);
  } catch (err) {
    throw StateError.ddlError(err instanceof Error ? err : new Error(String(err)));
  }
  return lockKey;
}

/// Release a PostgreSQL advisory lock
async function releaseDdlLock(client: PostgresClient, lockKey: bigint): Promise<void> {
  try {
    await client.query('SELECT pg_advisory_unlock($1)', [lockKey]);
  } catch (err) {
    throw StateError.ddlError(err instanceof Error ? err : new Error(String(err)));
  }
}

// ── PostgresColumn ───────────────────────────────────────────────────

export interface PostgresColumn {
  name: string;
  isNullable: boolean;
  dataType: string;
}

// ── PostgresBucket ───────────────────────────────────────────────────

export class PostgresBucket implements StorageCollection {
  private pool: PostgresPool;
  private collectionId: CollectionId;
  private schema: string;
  private columns: PostgresColumn[] = [];
  /// Tracks the last predicate that spilled to post-filtering (debug/test only)
  private _lastSpilledPredicate: Predicate | null = null;

  constructor(
    pool: PostgresPool,
    collectionId: CollectionId,
    schema: string,
  ) {
    this.pool = pool;
    this.collectionId = collectionId;
    this.schema = schema;
  }

  private stateTable(): string {
    return this.collectionId.asStr();
  }

  eventTable(): string {
    return `${this.collectionId.asStr()}_event`;
  }

  /// Returns the last predicate that spilled to post-filtering.
  lastSpilledPredicate(): Predicate | null {
    return this._lastSpilledPredicate;
  }

  /// Rebuild the cache of columns in the table.
  async rebuildColumnsCache(client: PostgresClient): Promise<void> {
    const columnQuery =
      'SELECT column_name, is_nullable, data_type FROM information_schema.columns WHERE table_catalog = $1 AND table_name = $2';

    const result = await client.query(columnQuery, [this.schema, this.collectionId.asStr()]);
    const newColumns: PostgresColumn[] = [];
    for (const row of result.rows) {
      const isNullable = row['is_nullable'] as string;
      newColumns.push({
        name: row['column_name'] as string,
        isNullable: isNullable === 'YES',
        dataType: row['data_type'] as string,
      });
    }

    this.columns = newColumns;
  }

  existingColumns(): string[] {
    return this.columns.map((col) => col.name);
  }

  column(columnName: string): PostgresColumn | null {
    return this.columns.find((col) => col.name === columnName) ?? null;
  }

  hasColumn(columnName: string): boolean {
    return this.column(columnName) !== null;
  }

  async createEventTable(client: PostgresClient): Promise<void> {
    const createQuery = `CREATE TABLE IF NOT EXISTS "${this.eventTable()}"(
      "id" character(43) PRIMARY KEY,
      "entity_id" character(22),
      "operations" bytea,
      "parent" character(43)[],
      "attestations" bytea
    )`;

    try {
      await client.query(createQuery, []);
    } catch (err) {
      throw StateError.ddlError(err instanceof Error ? err : new Error(String(err)));
    }
  }

  async createStateTable(client: PostgresClient): Promise<void> {
    const createQuery = `CREATE TABLE IF NOT EXISTS "${this.stateTable()}"(
      "id" character(22) PRIMARY KEY,
      "state_buffer" BYTEA,
      "head" character(43)[],
      "attestations" BYTEA[]
    )`;

    try {
      await client.query(createQuery, []);
    } catch (err) {
      throw StateError.ddlError(err instanceof Error ? err : new Error(String(err)));
    }
  }

  async addMissingColumns(
    client: PostgresClient,
    missing: Array<[string, string]>, // [column name, datatype]
  ): Promise<void> {
    if (missing.length === 0) {
      return;
    }

    // Acquire advisory lock to serialize DDL operations for this collection
    const lockKey = await acquireDdlLock(client, this.collectionId.asStr());

    try {
      // Re-check columns after acquiring lock (another session may have added them)
      await this.rebuildColumnsCache(client);

      for (const [column, datatype] of missing) {
        if (Postgres.saneName(column) && !this.hasColumn(column)) {
          const alterQuery = `ALTER TABLE "${this.stateTable()}" ADD COLUMN "${column}" ${datatype}`;
          try {
            await client.query(alterQuery, []);
          } catch (err) {
            await this.rebuildColumnsCache(client);
            throw StateError.ddlError(err instanceof Error ? err : new Error(String(err)));
          }
        }
      }

      await this.rebuildColumnsCache(client);
    } finally {
      // Always release the lock
      await releaseDdlLock(client, lockKey);
    }
  }

  // ── StorageCollection impl ───────────────────────────────────────

  async setState(state: Attested<EntityState>): Promise<boolean> {
    // Divergence: Rust uses bincode::serialize for state_buffers. TS would need
    // the same bincode codec. For now, we serialize with the proto codec [E8].
    // TODO: Implement bincode serialization for state_buffers when proto codec is ready.

    const client = await this.pool.getClient();

    const id = state.payload.entityId;
    const stateBuffers = serializeStateBuffers(state.payload.state.stateBuffers);
    const headArray = state.payload.state.head.toVec().map((eid) => eid.toBase64());
    const attestations = state.attestations.attestations.map((att: Attestation) => serializeAttestation(att));

    const columns: string[] = ['id', 'state_buffer', 'head', 'attestations'];
    const params: unknown[] = [id.toBase64(), stateBuffers, headArray, attestations];

    const materialized: Array<[string, PGValue | null]> = [];
    const seenProperties = new Set<string>();

    // Process property values directly from state buffers
    for (const [name, stateBuffer] of state.payload.state.stateBuffers.entries()) {
      const backend = backendFromString(name, stateBuffer);
      for (const [column, value] of backend.propertyValues()) {
        if (seenProperties.has(column)) {
          continue;
        }
        seenProperties.add(column);

        const pgValue: PGValue | null = value !== null ? pgValueFromValue(value) : null;
        if (!this.hasColumn(column)) {
          if (pgValue !== null) {
            await this.addMissingColumns(client, [[column, pgValuePostgresType(pgValue)]]);
          } else {
            continue;
          }
        }

        materialized.push([column, pgValue]);
      }
    }

    for (const [name, pgValue] of materialized) {
      columns.push(name);
      if (pgValue !== null) {
        params.push(extractPgValueParam(pgValue));
      } else {
        params.push(null);
      }
    }

    const columnsStr = columns.map((name) => `"${name}"`).join(', ');
    const valuesStr = params.map((_, index) => `$${index + 1}`).join(', ');
    const columnsUpdateStr = columns
      .slice(1) // Skip "id"
      .map((name, index) => `"${name}" = $${index + 2}`)
      .join(', ');

    const query = `WITH old_state AS (
      SELECT "head" FROM "${this.stateTable()}" WHERE "id" = $1
    )
    INSERT INTO "${this.stateTable()}"(${columnsStr}) VALUES(${valuesStr})
    ON CONFLICT("id") DO UPDATE SET ${columnsUpdateStr}
    RETURNING (SELECT "head" FROM old_state) as old_head`;

    const row = await client.queryOne(query, params);

    // If this is a new entity (no old_head), or if the heads are different, return true
    const oldHead = row['old_head'] as string[] | null;
    if (oldHead === null) {
      return true; // New entity
    }
    // Compare head arrays
    const newHead = headArray;
    if (oldHead.length !== newHead.length) return true;
    for (let i = 0; i < oldHead.length; i++) {
      if (oldHead[i] !== newHead[i]) return true;
    }
    return false;
  }

  async getState(id: EntityId): Promise<Attested<EntityState>> {
    const query = `SELECT "id", "state_buffer", "head", "attestations" FROM "${this.stateTable()}" WHERE "id" = $1`;

    const client = await this.pool.getClient();
    const result = await client.query(query, [id.toBase64()]);

    if (result.rows.length === 0) {
      throw RetrievalError.entityNotFound(id);
    }

    const row = result.rows[0];
    return deserializeEntityStateRow(row, this.collectionId);
  }

  async fetchStates(selection: Selection): Promise<Attested<EntityState>[]> {
    const client = await this.pool.getClient();

    // Pre-filter selection based on cached schema to avoid undefined column errors.
    const referenced = selection.referencedColumns();
    const cached = this.existingColumns();
    const unknownToCache = referenced.filter((col) => !cached.includes(col));

    // Refresh cache if we see columns we haven't seen before
    if (unknownToCache.length > 0) {
      await this.rebuildColumnsCache(client);
    }

    // Now check with (possibly refreshed) cache - columns still missing truly don't exist
    const existing = this.existingColumns();
    const missing = referenced.filter((col) => !existing.includes(col));

    const effectiveSelection = missing.length === 0
      ? selection
      : selection.assumeNull(missing);

    // Split predicate into parts we can pushdown to PostgreSQL vs post-filter in TS
    const split = splitPredicateForPostgres(effectiveSelection.predicate);
    const needsPostFilter = split.needsPostFilter();
    const remainingPredicate = split.remainingPredicate;

    // Track spilled predicate for test assertions
    this._lastSpilledPredicate = needsPostFilter ? remainingPredicate : null;

    // Build SQL with only the pushdown-capable predicate
    const sqlSelection = new Selection(
      split.sqlPredicate,
      effectiveSelection.orderBy,
      needsPostFilter
        ? null // Can't limit in SQL if we need to post-filter (would drop valid results)
        : effectiveSelection.limit,
    );

    const builder = SqlBuilder.withFields(['id', 'state_buffer', 'head', 'attestations']);
    builder.tableName(this.stateTable());
    builder.selection(sqlSelection);

    const { sql, args } = builder.build();

    const result = await client.query(sql, args);

    let results: Attested<EntityState>[] = result.rows.map((row) =>
      deserializeEntityStateRow(row, this.collectionId),
    );

    // Post-filter results if we have remaining predicate that couldn't be pushed down
    if (needsPostFilter) {
      results = postFilterStates(results, remainingPredicate, this.collectionId);

      // Apply limit after post-filter if needed
      if (effectiveSelection.limit !== null) {
        results = results.slice(0, effectiveSelection.limit);
      }
    }

    return results;
  }

  async addEvent(entityEvent: Attested<Event>): Promise<boolean> {
    const operations = serializeOperations(entityEvent.payload.operations);
    const attestations = serializeAttestationSet(entityEvent.attestations);
    const parentArray = entityEvent.payload.parent.toVec().map((eid) => eid.toBase64());

    const query = `INSERT INTO "${this.eventTable()}"("id", "entity_id", "operations", "parent", "attestations") VALUES($1, $2, $3, $4, $5)
      ON CONFLICT ("id") DO NOTHING`;

    const client = await this.pool.getClient();

    const result = await client.query(query, [
      entityEvent.payload.id().toBase64(),
      entityEvent.payload.entityId.toBase64(),
      operations,
      parentArray,
      attestations,
    ]);

    return result.rowCount > 0;
  }

  async getEvents(eventIds: EventId[]): Promise<Attested<Event>[]> {
    if (eventIds.length === 0) {
      return [];
    }

    const query = `SELECT "id", "entity_id", "operations", "parent", "attestations" FROM "${this.eventTable()}" WHERE "id" = ANY($1)`;

    const client = await this.pool.getClient();
    const idStrings = eventIds.map((id) => id.toBase64());
    const result = await client.query(query, [idStrings]);

    return result.rows.map((row) => deserializeEventRow(row, this.collectionId));
  }

  async dumpEntityEvents(entityId: EntityId): Promise<Attested<Event>[]> {
    const query = `SELECT "id", "operations", "parent", "attestations" FROM "${this.eventTable()}" WHERE "entity_id" = $1`;

    const client = await this.pool.getClient();
    const result = await client.query(query, [entityId.toBase64()]);

    return result.rows.map((row) => deserializeEventRow(row, this.collectionId, entityId));
  }
}

// ── ErrorKind ────────────────────────────────────────────────────────
// Mirrors Rust's ErrorKind enum for postgres error classification.

export type ErrorKind =
  | { type: 'RowCount' }
  | { type: 'UndefinedTable'; table: string }
  | { type: 'UndefinedColumn'; table: string | null; column: string }
  | { type: 'Unknown' }
  | { type: 'PostgresError'; message: string };

/// Classify a PostgreSQL error into an ErrorKind.
/// Divergence: Rust parses tokio_postgres::Error with SqlState codes.
/// TS inspects error messages since PG client libraries vary [E8].
export function errorKind(err: unknown): ErrorKind {
  const message = err instanceof Error ? err.message : String(err);

  // Check for RowCount errors (client-side)
  if (message.includes('query returned an unexpected number of rows')) {
    return { type: 'RowCount' };
  }

  // Check for undefined table (SQLSTATE 42P01)
  const undefinedTableMatch = message.match(/relation "([^"]+)" does not exist/);
  if (undefinedTableMatch) {
    return { type: 'UndefinedTable', table: undefinedTableMatch[1] };
  }

  // Check for undefined column (SQLSTATE 42703)
  const undefinedColMatch = message.match(/column "([^"]+)".*does not exist/);
  if (undefinedColMatch) {
    const column = undefinedColMatch[1];
    const tableMatch = message.match(/of relation "([^"]+)"/);
    return { type: 'UndefinedColumn', table: tableMatch ? tableMatch[1] : null, column };
  }

  return { type: 'Unknown' };
}

// ── MissingMaterialized ──────────────────────────────────────────────
// Rust: `pub struct MissingMaterialized` (unused)

// ── post_filter_states ───────────────────────────────────────────────

/// Post-filter EntityStates using a predicate that couldn't be pushed to SQL.
function postFilterStates(
  states: Attested<EntityState>[],
  predicate: Predicate,
  _collectionId: CollectionId,
): Attested<EntityState>[] {
  // Divergence: Rust uses TemporaryEntity (not yet ported). TS builds a minimal
  // Filterable adapter from the EntityState [E8].
  return states.filter((attested) => {
    try {
      const filterable = entityStateAsFilterable(attested, _collectionId);
      return evaluatePredicate(filterable, predicate);
    } catch {
      return false; // Exclude entities that fail evaluation
    }
  });
}

/// Create a Filterable adapter from an Attested<EntityState>.
/// Mirrors Rust TemporaryEntity which implements Filterable.
function entityStateAsFilterable(attested: Attested<EntityState>, collectionId: CollectionId): Filterable {
  // Build a property value map from state buffers
  const values = new Map<string, import('@ankurah/core').Value | null>();
  for (const [name, buffer] of attested.payload.state.stateBuffers.entries()) {
    try {
      const backend = backendFromString(name, buffer);
      for (const [prop, value] of backend.propertyValues()) {
        values.set(prop, value);
      }
    } catch {
      // Skip backends that fail to deserialize
    }
  }

  return {
    collection(): string {
      return collectionId.asStr();
    },
    value(name: string) {
      return values.get(name) ?? null;
    },
  };
}

// ── Serialization helpers ────────────────────────────────────────────
// Mirrors Rust bincode::serialize / bincode::deserialize calls in the Rust postgres engine.

/// Ensure a value is a Uint8Array (pg driver may return Buffer).
function toUint8Array(value: unknown): Uint8Array {
  if (value instanceof Uint8Array) return value;
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  // node-postgres returns Buffer for bytea columns; Buffer extends Uint8Array in Node
  if (Buffer.isBuffer(value)) return new Uint8Array(value);
  throw new Error(`Expected Uint8Array or Buffer, got ${typeof value}`);
}

/// Serialize StateBuffers to a single bytea blob via bincode.
/// Rust: `bincode::serialize(&state.payload.state.state_buffers)?`
function serializeStateBuffers(stateBuffers: StateBuffers): Uint8Array {
  const writer = new BincodeWriter();
  stateBuffers.encode(writer);
  return writer.finish();
}

/// Serialize a single Attestation to a bytea blob via bincode.
/// Rust: `state.attestations.iter().map(bincode::serialize)`
function serializeAttestation(attestation: Attestation): Uint8Array {
  const writer = new BincodeWriter();
  attestation.encode(writer);
  return writer.finish();
}

/// Serialize an AttestationSet to a single bytea blob via bincode.
/// Rust: `bincode::serialize(&entity_event.attestations)?`
function serializeAttestationSet(attestations: AttestationSet): Uint8Array {
  const writer = new BincodeWriter();
  attestations.encode(writer);
  return writer.finish();
}

/// Serialize OperationSet to a single bytea blob via bincode.
/// Rust: `bincode::serialize(&entity_event.payload.operations)?`
function serializeOperations(operations: OperationSet): Uint8Array {
  const writer = new BincodeWriter();
  operations.encode(writer);
  return writer.finish();
}

/// Deserialize a state row from the state table into Attested<EntityState>.
function deserializeEntityStateRow(row: Record<string, unknown>, collectionId: CollectionId): Attested<EntityState> {
  const id = EntityId.fromBase64(row['id'] as string);

  // Deserialize state_buffers from bytea
  const stateBufferBytes = toUint8Array(row['state_buffer']);
  const stateBuffers = StateBuffers.decode(new BincodeReader(stateBufferBytes));

  // Deserialize head from character(43)[] — postgres returns as string[]
  const headStrings = row['head'] as string[];
  const head = Clock.fromStrings(headStrings);

  // Deserialize attestations from BYTEA[] — each element is an individually serialized Attestation
  const attestationBuffers = row['attestations'] as unknown[];
  const attestations: Attestation[] = (attestationBuffers ?? []).map((buf) =>
    Attestation.decode(new BincodeReader(toUint8Array(buf))),
  );

  return new Attested(
    new EntityState(id, collectionId, new State(stateBuffers, head)),
    new AttestationSet(attestations),
  );
}

/// Deserialize an event row from the event table into Attested<Event>.
function deserializeEventRow(row: Record<string, unknown>, collectionId: CollectionId, entityId?: EntityId): Attested<Event> {
  // entity_id may come from the row or be passed in (dump_entity_events doesn't select it)
  const resolvedEntityId = entityId ?? EntityId.fromBase64(row['entity_id'] as string);

  // Deserialize operations from bytea
  const opsBytes = toUint8Array(row['operations']);
  const operations = OperationSet.decode(new BincodeReader(opsBytes));

  // Deserialize parent from character(43)[] — postgres returns as string[]
  const parentStrings = row['parent'] as string[];
  const parent = Clock.fromStrings(parentStrings);

  // Deserialize attestations from bytea (single blob for the whole AttestationSet)
  const attestationsBytes = toUint8Array(row['attestations']);
  const attestations = AttestationSet.decode(new BincodeReader(attestationsBytes));

  return new Attested(
    new Event(collectionId, resolvedEntityId, operations, parent),
    attestations,
  );
}

// ── PGValue param extraction ─────────────────────────────────────────

function extractPgValueParam(pgValue: PGValue): unknown {
  return pgValue.value;
}
