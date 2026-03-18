// MIRRORS: ankurah/storage/indexeddb-wasm/src/collection.rs

// Divergence: Rust uses wasm-bindgen + SendWrapper + async_trait + web_sys types. [E16]
// In TS, we use native IndexedDB API directly with Promises.

import type { StorageCollection, Filterable, Value, KeySpec } from '@ankurah/core';
import { evaluatePredicate, backendFromString, keySpecNameWith } from '@ankurah/core';
import { EntityId, type Attested, type CollectionId, type EntityState, type Event, type EventId } from '@ankurah/proto';
import { Selection, Predicate, ComparisonOperator, Expr, Literal, PathExpr } from '@ankurah/ankql';
import { RetrievalError, MutationError } from '@ankurah/core';
import {
  Planner, plannerConfigIndexeddb,
  type OrderByComponents,
  ScanDirection,
} from '@ankurah/storage-common';
import { sortBy, topK, type HasEntityId } from '@ankurah/storage-common';

import { Database } from './database.ts';
import {
  ID_KEY, HEAD_KEY, COLLECTION_KEY, STATE_BUFFER_KEY,
  ENTITY_ID_KEY, OPERATIONS_KEY, ATTESTATIONS_KEY, PARENT_KEY,
} from './statics.ts';
import { cbFuture } from './util/cb_future.ts';
import { IdbObject } from './util/object.ts';
import { valueToIdb, idbToValue } from './idb_value.ts';
import { IdbIndexScanner } from './scanner.ts';
import { planBoundsToIdbRange, scanDirectionToCursorDirection } from './planner_integration.ts';

export class IndexedDBBucket implements StorageCollection {
  readonly db: Database;
  readonly collectionId: CollectionId;
  private invocationCount: number = 0;
  /** For testing: disable prefix guard at runtime */
  prefixGuardDisabled: boolean = false;

  constructor(db: Database, collectionId: CollectionId) {
    this.db = db;
    this.collectionId = collectionId;
  }

  toString(): string {
    return `IndexedDBBucket(${this.collectionId})`;
  }

  async setState(state: Attested<EntityState>): Promise<boolean> {
    this.invocationCount++;

    const dbConnection = await this.db.getConnection();

    // Get the old entity if it exists to check for changes
    const transaction = dbConnection.transaction('entities', 'readwrite');
    const store = transaction.objectStore('entities');
    const oldRequest = store.get(state.payload.entityId.toString());
    await cbFuture(oldRequest, 'success', 'error');

    const oldEntity = oldRequest.result;

    // Check if the entity changed
    if (oldEntity !== undefined && oldEntity !== null) {
      const oldObj = new IdbObject(oldEntity);
      const oldHead = oldObj.getOpt(HEAD_KEY);
      if (oldHead !== undefined) {
        // Compare clocks — if head is the same, no update needed
        // Divergence: Rust deserializes Clock from IDB value; TS compares serialized form [E16]
        const oldHeadStr = JSON.stringify(oldHead);
        const newHeadStr = JSON.stringify(state.payload.state.head);
        if (oldHeadStr === newHeadStr) {
          return false;
        }
      }
    }

    const entity: Record<string, unknown> = {};
    entity[ID_KEY] = state.payload.entityId.toString();
    entity[COLLECTION_KEY] = this.collectionId.toString();
    entity[STATE_BUFFER_KEY] = state.payload.state.stateBuffers;
    entity[HEAD_KEY] = state.payload.state.head;
    entity[ATTESTATIONS_KEY] = state.attestations;

    // Extract all fields for indexing
    extractAllFields(entity, state.payload);

    // Put the entity in the store
    const putRequest = store.put(entity, state.payload.entityId.toString());
    await cbFuture(putRequest, 'success', 'error');
    await cbFuture(transaction, 'complete', 'error');

    return true;
  }

  async getState(id: EntityId): Promise<Attested<EntityState>> {
    const dbConnection = await this.db.getConnection();
    const transaction = dbConnection.transaction('entities', 'readonly');
    const store = transaction.objectStore('entities');
    const request = store.get(id.toString());
    await cbFuture(request, 'success', 'error');

    const result = request.result;
    if (result === undefined || result === null) {
      throw RetrievalError.entityNotFound(id);
    }

    const entity = new IdbObject(result);
    return idbObjectToEntityState(entity, this.collectionId);
  }

  async fetchStates(selection: Selection): Promise<Attested<EntityState>[]> {
    this.invocationCount++;

    // Step 1: Amend predicate with __collection comparison
    const amendedSelection = addCollection(selection, this.collectionId);

    // Step 2: Use planner to generate query plans
    const planner = new Planner(plannerConfigIndexeddb());
    const plans = planner.plan(amendedSelection, 'id');

    // Step 3: Pick the first plan
    const plan = plans[0];
    if (plan === undefined) {
      throw RetrievalError.storageError(new Error('No plan generated'));
    }

    return plan.match<Promise<Attested<EntityState>[]>>({
      EmptyScan: async () => [],

      Index: async (data) => {
        const { indexSpec, bounds, scanDirection, remainingPredicate, orderBySpill } = data;

        // Step 4: Ensure index exists
        await this.db.assureIndexExists(indexSpec);

        // Step 5: Execute the query using the plan
        const dbConnection = await this.db.getConnection();
        const transaction = dbConnection.transaction('entities', 'readonly');
        const store = transaction.objectStore('entities');
        const index = store.index(keySpecNameWith(indexSpec, '', '__'));

        // Convert plan bounds to IndexedDB key range
        const [keyRange, upperOpenEnded, eqPrefixLen, eqPrefixValues] =
          planBoundsToIdbRange(bounds, scanDirection);
        const cursorDirection = scanDirectionToCursorDirection(scanDirection);

        return this.executePlanQuery(
          index, keyRange, remainingPredicate, cursorDirection,
          selection.limit ?? null, upperOpenEnded, eqPrefixLen,
          eqPrefixValues, orderBySpill,
        );
      },

      TableScan: async () => {
        throw new Error(
          'We should always have an IndexPlan or EmptyScan due to the amendment of the selection to include the collection',
        );
      },
    });
  }

  async addEvent(attestedEvent: Attested<Event>): Promise<boolean> {
    this.invocationCount++;

    const dbConnection = await this.db.getConnection();
    const transaction = dbConnection.transaction('events', 'readwrite');
    const store = transaction.objectStore('events');

    const payload = attestedEvent.payload;
    const eventObj: Record<string, unknown> = {};
    eventObj[ID_KEY] = payload.id().toBase64();
    eventObj[ENTITY_ID_KEY] = payload.entityId.toBase64();
    eventObj[OPERATIONS_KEY] = payload.operations;
    eventObj[ATTESTATIONS_KEY] = attestedEvent.attestations;
    eventObj[PARENT_KEY] = payload.parent;

    const request = store.put(eventObj, payload.id().toBase64());
    await cbFuture(request, 'success', 'error');
    await cbFuture(transaction, 'complete', 'error');

    return true;
  }

  async getEvents(eventIds: EventId[]): Promise<Attested<Event>[]> {
    if (eventIds.length === 0) {
      return [];
    }

    const dbConnection = await this.db.getConnection();
    const transaction = dbConnection.transaction('events', 'readonly');
    const store = transaction.objectStore('events');

    const events: Attested<Event>[] = [];
    for (const eventId of eventIds) {
      const request = store.get(eventId.toBase64());
      await cbFuture(request, 'success', 'error');
      const result = request.result;

      if (result === undefined || result === null) {
        continue;
      }

      const eventObj = new IdbObject(result);
      events.push(idbObjectToEvent(eventObj, this.collectionId));
    }

    return events;
  }

  async dumpEntityEvents(id: EntityId): Promise<Attested<Event>[]> {
    const dbConnection = await this.db.getConnection();
    const transaction = dbConnection.transaction('events', 'readonly');
    const store = transaction.objectStore('events');
    const index = store.index('by_entity_id');
    const keyRange = IDBKeyRange.only(id.toBase64());
    const request = index.openCursor(keyRange);

    const events: Attested<Event>[] = [];

    while (true) {
      const cursor = await new Promise<IDBCursorWithValue | null>((resolve, reject) => {
        request.onsuccess = () => resolve(request.result as IDBCursorWithValue | null);
        request.onerror = () => reject(new Error(`Cursor error: ${request.error?.message}`));
      });

      if (cursor === null) break;

      const eventObj = new IdbObject(cursor.value);
      events.push(idbObjectToEvent(eventObj, this.collectionId));

      cursor.continue();
    }

    return events;
  }

  // ── Private: execute_plan_query ──────────────────────────────────────

  private async executePlanQuery(
    index: IDBIndex,
    keyRange: IDBKeyRange,
    predicate: Predicate,
    cursorDirection: IDBCursorDirection,
    limit: number | null,
    upperOpenEnded: boolean,
    eqPrefixLen: number,
    eqPrefixValues: Value[],
    orderBySpill: OrderByComponents,
  ): Promise<Attested<EntityState>[]> {
    const needsSpillSort = orderBySpill.spill.length > 0;

    // Determine effective prefix guard config
    const effectivePrefixLen =
      (upperOpenEnded && eqPrefixLen > 0 && !this.prefixGuardDisabled)
        ? eqPrefixLen
        : 0;

    // Use IdbIndexScanner for cursor iteration with prefix guard
    const scanner = new IdbIndexScanner(
      index, keyRange, cursorDirection, effectivePrefixLen, eqPrefixValues,
    );

    let count = 0;
    const rows: IdbRecord[] = [];
    const directResults: Attested<EntityState>[] = [];

    for await (const entityObj of scanner.scan()) {
      // Create IdbRecord - wraps JS object with lazy value extraction
      let record: IdbRecord;
      try {
        record = IdbRecord.create(entityObj, this.collectionId);
      } catch {
        continue;
      }

      // Apply predicate filtering
      if (evaluatePredicate(record, predicate)) {
        if (needsSpillSort) {
          rows.push(record);
        } else {
          // No sorting needed - extract entity state and apply limit during scan
          try {
            directResults.push(record.entityState());
            count++;
            if (limit !== null && count >= limit) {
              break;
            }
          } catch {
            // Skip records that fail to convert
          }
        }
      }
    }

    // If we need to sort by spilled columns, use partition-aware sorting
    if (needsSpillSort) {
      const asAsync = async function* () { yield* rows; };
      const results: Attested<EntityState>[] = [];

      if (limit !== null) {
        for await (const r of topK(asAsync(), orderBySpill, limit)) {
          try { results.push(r.entityState()); } catch { /* skip */ }
        }
      } else {
        for await (const r of sortBy(asAsync(), orderBySpill)) {
          try { results.push(r.entityState()); } catch { /* skip */ }
        }
      }
      return results;
    }

    return directResults;
  }
}

// ── IdbRecord ─────────────────────────────────────────────────────────

/**
 * A record from the IndexedDB entities store.
 * Wraps the raw JS object with lazy extraction for filtering and sorting.
 * Implements Filterable and HasEntityId for use with stream combinators.
 */
class IdbRecord implements Filterable, HasEntityId {
  private readonly id_: EntityId;
  private readonly object: IdbObject;
  private readonly collectionId_: CollectionId;

  private constructor(id: EntityId, object: IdbObject, collectionId: CollectionId) {
    this.id_ = id;
    this.object = object;
    this.collectionId_ = collectionId;
  }

  static create(object: IdbObject, collectionId: CollectionId): IdbRecord {
    // EntityId is stored as a string; need to parse it back
    const idStr = object.get(ID_KEY) as string;
    // Divergence: Rust uses TryFrom<JsValue> for EntityId; TS parses from string [E16]
    const id = EntityId.fromBase64(idStr);
    return new IdbRecord(id, object, collectionId);
  }

  /** Get the entity state (converts from JS object on demand) */
  entityState(): Attested<EntityState> {
    return idbObjectToEntityState(this.object, this.collectionId_);
  }

  // Filterable interface
  collection(): string {
    return this.collectionId_.toString();
  }

  value(name: string): Value | null {
    return this.object.getValueOpt(name) ?? null;
  }

  // HasEntityId interface
  entityId(): EntityId {
    return this.id_;
  }
}

// ── Helper functions ──────────────────────────────────────────────────

/** Convert IdbObject to EntityState */
function idbObjectToEntityState(
  entityObj: IdbObject,
  collectionId: CollectionId,
): Attested<EntityState> {
  const idStr = entityObj.get(ID_KEY) as string;
  const id = EntityId.fromBase64(idStr);

  return {
    payload: {
      collection: collectionId,
      entityId: id,
      state: {
        stateBuffers: entityObj.get(STATE_BUFFER_KEY),
        head: entityObj.get(HEAD_KEY),
      },
    },
    attestations: entityObj.get(ATTESTATIONS_KEY),
  } as Attested<EntityState>;
}

/** Convert IdbObject to Event */
function idbObjectToEvent(
  eventObj: IdbObject,
  collectionId: CollectionId,
): Attested<Event> {
  return {
    payload: {
      collection: collectionId,
      entityId: eventObj.get(ENTITY_ID_KEY),
      operations: eventObj.get(OPERATIONS_KEY),
      parent: eventObj.get(PARENT_KEY),
    },
    attestations: eventObj.get(ATTESTATIONS_KEY),
  } as Attested<Event>;
}

/** Extract all fields from entity state and set them directly on the IndexedDB entity object */
function extractAllFields(entity: Record<string, unknown>, entityState: EntityState): void {
  const seenFields = new Set<string>();

  // Process all property values from state buffers
  for (const [backendName, stateBuffer] of entityState.state.stateBuffers) {
    const backend = backendFromString(backendName, stateBuffer);

    for (const [fieldName, value] of backend.propertyValues()) {
      // Use first occurrence (like Postgres) to handle field name collisions
      if (seenFields.has(fieldName)) {
        continue;
      }
      seenFields.add(fieldName);

      // Set field directly on entity object (no prefix — they become the primary fields)
      // Use IdbValue encoding to ensure fields are IndexedDB-key-compatible (bool as 0/1, etc.)
      entity[fieldName] = value !== null ? valueToIdb(value) : null;
    }
  }
}

/**
 * Amend a selection with __collection = 'value' comparison.
 *
 * Mirrors Rust `add_collection(selection, collection_id)`.
 */
export function addCollection(selection: Selection, collectionId: CollectionId): Selection {
  const collectionComparison = Predicate.Comparison(
    Expr.Path(PathExpr.simple('__collection')),
    ComparisonOperator.Equal(),
    Expr.Literal(Literal.String(collectionId.toString())),
  );

  return new Selection(
    Predicate.And(collectionComparison, selection.predicate),
    selection.orderBy,
    selection.limit,
  );
}
