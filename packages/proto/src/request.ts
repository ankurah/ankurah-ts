// MIRRORS: ankurah/proto/src/request.rs

import { BincodeReader, BincodeWriter } from './codec';
import { AuthData, Attested, AttestationSet } from './auth';
import { Clock } from './clock';
import { CollectionId } from './collection';
import {
  EntityId, EventId, RequestId, TransactionId, QueryId,
} from './id';
import {
  Event, EventFragment, StateFragment, EntityState,
} from './data';

// ─── Forward declarations for ankql AST types ──────────────────────────────
// These will be properly imported from @ankurah/ankql when that package is implemented.
// For now, we define opaque encode/decode that preserves wire compatibility.
// The actual AST type definitions are in @ankurah/ankql.

/**
 * Opaque placeholder for ankql::ast::Selection.
 * Stores the raw bincode bytes so we can round-trip without understanding the structure.
 * This will be replaced with proper type imports from @ankurah/ankql.
 */
export class OpaqueSelection {
  readonly rawBytes: Uint8Array;

  constructor(rawBytes: Uint8Array) {
    this.rawBytes = rawBytes;
  }

  encode(writer: BincodeWriter): void {
    writer.writeRawBytes(this.rawBytes);
  }

  // NOTE: OpaqueSelection cannot self-decode because it doesn't know its own length.
  // Callers must track positions or use a proper Selection decoder from @ankurah/ankql.
  // For now, request/response types that contain Selection store it as raw bytes.
}

// ─── KnownEntity ────────────────────────────────────────────────────────────

/**
 * Entity with known head for lineage attestation.
 * Derived serde — struct { entity_id: EntityId, head: Clock }.
 */
export class KnownEntity {
  readonly entityId: EntityId;
  readonly head: Clock;

  constructor(entityId: EntityId, head: Clock) {
    this.entityId = entityId;
    this.head = head;
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.head.encode(writer);
  }

  static decode(reader: BincodeReader): KnownEntity {
    const entityId = EntityId.decode(reader);
    const head = Clock.decode(reader);
    return new KnownEntity(entityId, head);
  }
}

// ─── CausalRelation ─────────────────────────────────────────────────────────

/**
 * CausalRelation: relationship between two clocks.
 * Derived serde as enum (u32 variant index + fields).
 *
 * Variant indices:
 *   0 = Equal
 *   1 = StrictDescends
 *   2 = StrictAscends
 *   3 = DivergedSince { meet, subject, other }
 *   4 = Disjoint { gca, subject_root, other_root }
 *   5 = BudgetExceeded { subject, other }
 */
export type CausalRelation =
  | { type: 'Equal' }
  | { type: 'StrictDescends' }
  | { type: 'StrictAscends' }
  | { type: 'DivergedSince'; meet: Clock; subject: Clock; other: Clock }
  | { type: 'Disjoint'; gca: Clock | null; subjectRoot: EventId; otherRoot: EventId }
  | { type: 'BudgetExceeded'; subject: Clock; other: Clock };

export function encodeCausalRelation(writer: BincodeWriter, rel: CausalRelation): void {
  switch (rel.type) {
    case 'Equal':
      writer.writeVariant(0);
      break;
    case 'StrictDescends':
      writer.writeVariant(1);
      break;
    case 'StrictAscends':
      writer.writeVariant(2);
      break;
    case 'DivergedSince':
      writer.writeVariant(3);
      rel.meet.encode(writer);
      rel.subject.encode(writer);
      rel.other.encode(writer);
      break;
    case 'Disjoint':
      writer.writeVariant(4);
      writer.writeOption(rel.gca, (w, c) => c.encode(w));
      rel.subjectRoot.encode(writer);
      rel.otherRoot.encode(writer);
      break;
    case 'BudgetExceeded':
      writer.writeVariant(5);
      rel.subject.encode(writer);
      rel.other.encode(writer);
      break;
  }
}

export function decodeCausalRelation(reader: BincodeReader): CausalRelation {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: return { type: 'Equal' };
    case 1: return { type: 'StrictDescends' };
    case 2: return { type: 'StrictAscends' };
    case 3: {
      const meet = Clock.decode(reader);
      const subject = Clock.decode(reader);
      const other = Clock.decode(reader);
      return { type: 'DivergedSince', meet, subject, other };
    }
    case 4: {
      const gca = reader.readOption(r => Clock.decode(r));
      const subjectRoot = EventId.decode(reader);
      const otherRoot = EventId.decode(reader);
      return { type: 'Disjoint', gca, subjectRoot, otherRoot };
    }
    case 5: {
      const subject = Clock.decode(reader);
      const other = Clock.decode(reader);
      return { type: 'BudgetExceeded', subject, other };
    }
    default:
      throw new Error(`Unknown CausalRelation variant: ${variant}`);
  }
}

// ─── CausalAssertion ────────────────────────────────────────────────────────

/**
 * CausalAssertion: not sent over the wire, but used for lineage validation.
 * Derived serde — struct { entity_id, subject, other, relation }.
 */
export class CausalAssertion {
  readonly entityId: EntityId;
  readonly subject: Clock;
  readonly other: Clock;
  readonly relation: CausalRelation;

  constructor(entityId: EntityId, subject: Clock, other: Clock, relation: CausalRelation) {
    this.entityId = entityId;
    this.subject = subject;
    this.other = other;
    this.relation = relation;
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.subject.encode(writer);
    this.other.encode(writer);
    encodeCausalRelation(writer, this.relation);
  }

  static decode(reader: BincodeReader): CausalAssertion {
    const entityId = EntityId.decode(reader);
    const subject = Clock.decode(reader);
    const other = Clock.decode(reader);
    const relation = decodeCausalRelation(reader);
    return new CausalAssertion(entityId, subject, other, relation);
  }
}

// ─── CausalAssertionFragment ────────────────────────────────────────────────

/**
 * Wire-minimal lineage attestation.
 * Derived serde — struct { relation, attestations }.
 */
export class CausalAssertionFragment {
  readonly relation: CausalRelation;
  readonly attestations: AttestationSet;

  constructor(relation: CausalRelation, attestations: AttestationSet) {
    this.relation = relation;
    this.attestations = attestations;
  }

  encode(writer: BincodeWriter): void {
    encodeCausalRelation(writer, this.relation);
    this.attestations.encode(writer);
  }

  static decode(reader: BincodeReader): CausalAssertionFragment {
    const relation = decodeCausalRelation(reader);
    const attestations = AttestationSet.decode(reader);
    return new CausalAssertionFragment(relation, attestations);
  }
}

// ─── DeltaContent ───────────────────────────────────────────────────────────

/**
 * DeltaContent: content for entity initialization.
 * Derived serde as enum.
 *
 * Variant indices:
 *   0 = StateSnapshot { state: StateFragment }
 *   1 = EventBridge { events: Vec<EventFragment> }
 *   2 = StateAndRelation { state: StateFragment, relation: CausalAssertionFragment }
 */
export type DeltaContent =
  | { type: 'StateSnapshot'; state: StateFragment }
  | { type: 'EventBridge'; events: EventFragment[] }
  | { type: 'StateAndRelation'; state: StateFragment; relation: CausalAssertionFragment };

export function encodeDeltaContent(writer: BincodeWriter, content: DeltaContent): void {
  switch (content.type) {
    case 'StateSnapshot':
      writer.writeVariant(0);
      content.state.encode(writer);
      break;
    case 'EventBridge':
      writer.writeVariant(1);
      writer.writeVec(content.events, (w, e) => e.encode(w));
      break;
    case 'StateAndRelation':
      writer.writeVariant(2);
      content.state.encode(writer);
      content.relation.encode(writer);
      break;
  }
}

export function decodeDeltaContent(reader: BincodeReader): DeltaContent {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: {
      const state = StateFragment.decode(reader);
      return { type: 'StateSnapshot', state };
    }
    case 1: {
      const events = reader.readVec(r => EventFragment.decode(r));
      return { type: 'EventBridge', events };
    }
    case 2: {
      const state = StateFragment.decode(reader);
      const relation = CausalAssertionFragment.decode(reader);
      return { type: 'StateAndRelation', state, relation };
    }
    default:
      throw new Error(`Unknown DeltaContent variant: ${variant}`);
  }
}

// ─── EntityDelta ────────────────────────────────────────────────────────────

/**
 * EntityDelta: entity initialization data returned in QuerySubscribed and Fetch.
 * Derived serde — struct { entity_id, collection, content }.
 */
export class EntityDelta {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly content: DeltaContent;

  constructor(entityId: EntityId, collection: CollectionId, content: DeltaContent) {
    this.entityId = entityId;
    this.collection = collection;
    this.content = content;
  }

  toString(): string {
    switch (this.content.type) {
      case 'StateSnapshot':
        return `EntityDelta ${this.entityId}: StateSnapshot(${this.content.state})`;
      case 'EventBridge':
        return `EntityDelta ${this.entityId}: EventBridge(${this.content.events.length} events)`;
      case 'StateAndRelation':
        return `EntityDelta ${this.entityId}: StateAndRelation(${this.content.state})`;
    }
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.collection.encode(writer);
    encodeDeltaContent(writer, this.content);
  }

  static decode(reader: BincodeReader): EntityDelta {
    const entityId = EntityId.decode(reader);
    const collection = CollectionId.decode(reader);
    const content = decodeDeltaContent(reader);
    return new EntityDelta(entityId, collection, content);
  }
}

// ─── NodeRequest ────────────────────────────────────────────────────────────

/**
 * NodeRequest: a request from one node to another.
 * Derived serde — struct { id, to, from, body }.
 */
export class NodeRequest {
  readonly id: RequestId;
  readonly to: EntityId;
  readonly from: EntityId;
  readonly body: NodeRequestBody;

  constructor(id: RequestId, to: EntityId, from: EntityId, body: NodeRequestBody) {
    this.id = id;
    this.to = to;
    this.from = from;
    this.body = body;
  }

  toString(): string {
    return `Request ${this.id} from ${this.from}->${this.to}: ${nodeRequestBodyToString(this.body)}`;
  }

  encode(writer: BincodeWriter): void {
    this.id.encode(writer);
    this.to.encode(writer);
    this.from.encode(writer);
    encodeNodeRequestBody(writer, this.body);
  }

  static decode(reader: BincodeReader): NodeRequest {
    const id = RequestId.decode(reader);
    const to = EntityId.decode(reader);
    const from = EntityId.decode(reader);
    const body = decodeNodeRequestBody(reader);
    return new NodeRequest(id, to, from, body);
  }
}

// ─── NodeRequestBody ────────────────────────────────────────────────────────

/**
 * NodeRequestBody enum.
 * Derived serde as enum.
 *
 * Variant indices:
 *   0 = CommitTransaction { id: TransactionId, events: Vec<Attested<Event>> }
 *   1 = Get { collection: CollectionId, ids: Vec<EntityId> }
 *   2 = GetEvents { collection: CollectionId, event_ids: Vec<EventId> }
 *   3 = Fetch { collection, selection, known_matches }
 *   4 = SubscribeQuery { query_id, collection, selection, version, known_matches }
 */
export type NodeRequestBody =
  | { type: 'CommitTransaction'; id: TransactionId; events: Attested<Event>[] }
  | { type: 'Get'; collection: CollectionId; ids: EntityId[] }
  | { type: 'GetEvents'; collection: CollectionId; eventIds: EventId[] }
  | { type: 'Fetch'; collection: CollectionId; selection: Uint8Array; knownMatches: KnownEntity[] }
  | { type: 'SubscribeQuery'; queryId: QueryId; collection: CollectionId; selection: Uint8Array; version: number; knownMatches: KnownEntity[] };
  // NOTE: selection is stored as raw bincode bytes until @ankurah/ankql provides proper encode/decode.

function encodeNodeRequestBody(writer: BincodeWriter, body: NodeRequestBody): void {
  switch (body.type) {
    case 'CommitTransaction':
      writer.writeVariant(0);
      body.id.encode(writer);
      writer.writeVec(body.events, (w, e) => {
        e.encode(w, (w2, event) => event.encode(w2));
      });
      break;
    case 'Get':
      writer.writeVariant(1);
      body.collection.encode(writer);
      writer.writeVec(body.ids, (w, id) => id.encode(w));
      break;
    case 'GetEvents':
      writer.writeVariant(2);
      body.collection.encode(writer);
      writer.writeVec(body.eventIds, (w, id) => id.encode(w));
      break;
    case 'Fetch':
      writer.writeVariant(3);
      body.collection.encode(writer);
      writer.writeRawBytes(body.selection);
      writer.writeVec(body.knownMatches, (w, km) => km.encode(w));
      break;
    case 'SubscribeQuery':
      writer.writeVariant(4);
      body.queryId.encode(writer);
      body.collection.encode(writer);
      writer.writeRawBytes(body.selection);
      writer.writeU32(body.version);
      writer.writeVec(body.knownMatches, (w, km) => km.encode(w));
      break;
  }
}

function decodeNodeRequestBody(reader: BincodeReader): NodeRequestBody {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: {
      const id = TransactionId.decode(reader);
      const events = reader.readVec(r => Attested.decode(r, r2 => Event.decode(r2)));
      return { type: 'CommitTransaction', id, events };
    }
    case 1: {
      const collection = CollectionId.decode(reader);
      const ids = reader.readVec(r => EntityId.decode(r));
      return { type: 'Get', collection, ids };
    }
    case 2: {
      const collection = CollectionId.decode(reader);
      const eventIds = reader.readVec(r => EventId.decode(r));
      return { type: 'GetEvents', collection, eventIds };
    }
    case 3: {
      const collection = CollectionId.decode(reader);
      // Selection is opaque bytes — we read it inline
      // TODO: replace with proper Selection.decode() from @ankurah/ankql
      const selection = decodeOpaqueBytes(reader);
      const knownMatches = reader.readVec(r => KnownEntity.decode(r));
      return { type: 'Fetch', collection, selection, knownMatches };
    }
    case 4: {
      const queryId = QueryId.decode(reader);
      const collection = CollectionId.decode(reader);
      const selection = decodeOpaqueBytes(reader);
      const version = reader.readU32();
      const knownMatches = reader.readVec(r => KnownEntity.decode(r));
      return { type: 'SubscribeQuery', queryId, collection, selection, version, knownMatches };
    }
    default:
      throw new Error(`Unknown NodeRequestBody variant: ${variant}`);
  }
}

/**
 * Placeholder: read opaque bytes for a type we cannot yet decode.
 * This reads ALL remaining bytes up to the next known boundary, which is WRONG.
 * TODO: When @ankurah/ankql is implemented, replace with Selection.decode().
 */
function decodeOpaqueBytes(_reader: BincodeReader): Uint8Array {
  // NOTE: This is a placeholder. The Selection type has a complex recursive structure
  // that cannot be opaquely skipped without knowing its size. Once @ankurah/ankql
  // provides Selection encode/decode, this will be replaced.
  throw new Error('Selection decode not yet implemented — requires @ankurah/ankql package');
}

function nodeRequestBodyToString(body: NodeRequestBody): string {
  switch (body.type) {
    case 'CommitTransaction':
      return `CommitTransaction ${body.id} [${body.events.length} events]`;
    case 'Get':
      return `Get ${body.collection} ${body.ids.map(id => id.toBase64Short()).join(', ')}`;
    case 'GetEvents':
      return `GetEvents ${body.collection} ${body.eventIds.map(id => id.toBase64Short()).join(', ')}`;
    case 'Fetch':
      return `Fetch ${body.collection} known:${body.knownMatches.length}`;
    case 'SubscribeQuery':
      return `Subscribe ${body.queryId} ${body.collection} v${body.version} known:${body.knownMatches.length}`;
  }
}

// ─── NodeResponse ───────────────────────────────────────────────────────────

/**
 * NodeResponse: a response from one node to another.
 * Derived serde — struct { request_id, from, to, body }.
 */
export class NodeResponse {
  readonly requestId: RequestId;
  readonly from: EntityId;
  readonly to: EntityId;
  readonly body: NodeResponseBody;

  constructor(requestId: RequestId, from: EntityId, to: EntityId, body: NodeResponseBody) {
    this.requestId = requestId;
    this.from = from;
    this.to = to;
    this.body = body;
  }

  toString(): string {
    return `Response(${this.requestId}) ${this.from}->${this.to} ${nodeResponseBodyToString(this.body)}`;
  }

  encode(writer: BincodeWriter): void {
    this.requestId.encode(writer);
    this.from.encode(writer);
    this.to.encode(writer);
    encodeNodeResponseBody(writer, this.body);
  }

  static decode(reader: BincodeReader): NodeResponse {
    const requestId = RequestId.decode(reader);
    const from = EntityId.decode(reader);
    const to = EntityId.decode(reader);
    const body = decodeNodeResponseBody(reader);
    return new NodeResponse(requestId, from, to, body);
  }
}

// ─── NodeResponseBody ───────────────────────────────────────────────────────

/**
 * NodeResponseBody enum.
 * Derived serde as enum.
 *
 * Variant indices:
 *   0 = CommitComplete { id: TransactionId }
 *   1 = Fetch(Vec<EntityDelta>)
 *   2 = Get(Vec<Attested<EntityState>>)
 *   3 = GetEvents(Vec<Attested<Event>>)
 *   4 = QuerySubscribed { query_id: QueryId, deltas: Vec<EntityDelta> }
 *   5 = Success
 *   6 = Error(String)
 */
export type NodeResponseBody =
  | { type: 'CommitComplete'; id: TransactionId }
  | { type: 'Fetch'; deltas: EntityDelta[] }
  | { type: 'Get'; states: Attested<EntityState>[] }
  | { type: 'GetEvents'; events: Attested<Event>[] }
  | { type: 'QuerySubscribed'; queryId: QueryId; deltas: EntityDelta[] }
  | { type: 'Success' }
  | { type: 'Error'; message: string };

function encodeNodeResponseBody(writer: BincodeWriter, body: NodeResponseBody): void {
  switch (body.type) {
    case 'CommitComplete':
      writer.writeVariant(0);
      body.id.encode(writer);
      break;
    case 'Fetch':
      writer.writeVariant(1);
      writer.writeVec(body.deltas, (w, d) => d.encode(w));
      break;
    case 'Get':
      writer.writeVariant(2);
      writer.writeVec(body.states, (w, s) => {
        s.encode(w, (w2, es) => es.encode(w2));
      });
      break;
    case 'GetEvents':
      writer.writeVariant(3);
      writer.writeVec(body.events, (w, e) => {
        e.encode(w, (w2, ev) => ev.encode(w2));
      });
      break;
    case 'QuerySubscribed':
      writer.writeVariant(4);
      body.queryId.encode(writer);
      writer.writeVec(body.deltas, (w, d) => d.encode(w));
      break;
    case 'Success':
      writer.writeVariant(5);
      break;
    case 'Error':
      writer.writeVariant(6);
      writer.writeString(body.message);
      break;
  }
}

function decodeNodeResponseBody(reader: BincodeReader): NodeResponseBody {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: {
      const id = TransactionId.decode(reader);
      return { type: 'CommitComplete', id };
    }
    case 1: {
      const deltas = reader.readVec(r => EntityDelta.decode(r));
      return { type: 'Fetch', deltas };
    }
    case 2: {
      const states = reader.readVec(r => Attested.decode(r, r2 => EntityState.decode(r2)));
      return { type: 'Get', states };
    }
    case 3: {
      const events = reader.readVec(r => Attested.decode(r, r2 => Event.decode(r2)));
      return { type: 'GetEvents', events };
    }
    case 4: {
      const queryId = QueryId.decode(reader);
      const deltas = reader.readVec(r => EntityDelta.decode(r));
      return { type: 'QuerySubscribed', queryId, deltas };
    }
    case 5:
      return { type: 'Success' };
    case 6: {
      const message = reader.readString();
      return { type: 'Error', message };
    }
    default:
      throw new Error(`Unknown NodeResponseBody variant: ${variant}`);
  }
}

function nodeResponseBodyToString(body: NodeResponseBody): string {
  switch (body.type) {
    case 'CommitComplete': return `CommitComplete ${body.id}`;
    case 'Fetch': return `Fetch [${body.deltas.length}]`;
    case 'Get': return `Get [${body.states.length}]`;
    case 'GetEvents': return `GetEvents [${body.events.length}]`;
    case 'QuerySubscribed': return `Subscribed ${body.queryId} initial:${body.deltas.length}`;
    case 'Success': return 'Success';
    case 'Error': return `Error: ${body.message}`;
  }
}
