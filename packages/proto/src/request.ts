// MIRRORS: ankurah/proto/src/request.rs

import { Struct, Enum } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { Attested, AttestationSet } from './auth';
import { Clock } from './clock';
import { CollectionId } from './collection';
import {
  Event, EventFragment, StateFragment, EntityState,
} from './data';
import {
  EntityId, EventId, RequestId, TransactionId, QueryId,
} from './id';

// Divergence: RequestId struct is defined in id.ts (co-located with other ULID IDs)
// rather than here. Imported from id.ts above. [E4]

// ─── NodeRequest ────────────────────────────────────────────────────────────

/// A request from one node to another
export class NodeRequest extends Struct {
  readonly id: RequestId;
  readonly to: EntityId;
  readonly from: EntityId;
  readonly body: NodeRequestBody;

  constructor(id: RequestId, to: EntityId, from: EntityId, body: NodeRequestBody) {
    super();
    this.id = id;
    this.to = to;
    this.from = from;
    this.body = body;
  }

  // impl Display for NodeRequest
  toString(): string {
    return `Request ${this.id} from ${this.from}->${this.to}: ${this.body}`;
  }

  encode(writer: BincodeWriter): void {
    this.id.encode(writer);
    this.to.encode(writer);
    this.from.encode(writer);
    this.body.encode(writer);
  }

  static decode(reader: BincodeReader): NodeRequest {
    const id = RequestId.decode(reader);
    const to = EntityId.decode(reader);
    const from = EntityId.decode(reader);
    const body = NodeRequestBody.decode(reader);
    return new NodeRequest(id, to, from, body);
  }
}

// ─── KnownEntity ────────────────────────────────────────────────────────────

/// Entity with known head for lineage attestation
export class KnownEntity extends Struct {
  readonly entityId: EntityId;
  readonly head: Clock;

  constructor(entityId: EntityId, head: Clock) {
    super();
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

// ─── EntityIdRange ──────────────────────────────────────────────────────────
// Divergence: EntityIdRange not in Rust — TS-ahead type for compact entity unsubscribe batches [E4]

/// Inclusive entity-id span used for compact unsubscribe batches.
export class EntityIdRange extends Struct {
  readonly start: EntityId;
  readonly end: EntityId;

  constructor(start: EntityId, end: EntityId) {
    super();
    this.start = start;
    this.end = end;
  }

  encode(writer: BincodeWriter): void {
    this.start.encode(writer);
    this.end.encode(writer);
  }

  static decode(reader: BincodeReader): EntityIdRange {
    const start = EntityId.decode(reader);
    const end = EntityId.decode(reader);
    return new EntityIdRange(start, end);
  }
}

// ─── CausalRelation ─────────────────────────────────────────────────────────

type CausalRelationV = {
  Equal: {};
  StrictDescends: {};
  StrictAscends: {};
  DivergedSince: { meet: Clock; subject: Clock; other: Clock };
  Disjoint: { gca: Clock | null; subjectRoot: EventId; otherRoot: EventId };
  BudgetExceeded: { subject: Clock; other: Clock };
};

export class CausalRelation extends Enum<CausalRelationV> {
  encode(writer: BincodeWriter): void {
    this.match({
      Equal: () => { writer.writeVariant(0); },
      StrictDescends: () => { writer.writeVariant(1); },
      StrictAscends: () => { writer.writeVariant(2); },
      DivergedSince: (v) => {
        writer.writeVariant(3);
        v.meet.encode(writer);
        v.subject.encode(writer);
        v.other.encode(writer);
      },
      Disjoint: (v) => {
        writer.writeVariant(4);
        writer.writeOption(v.gca, (w, c) => c.encode(w));
        v.subjectRoot.encode(writer);
        v.otherRoot.encode(writer);
      },
      BudgetExceeded: (v) => {
        writer.writeVariant(5);
        v.subject.encode(writer);
        v.other.encode(writer);
      },
    });
  }

  static decode(reader: BincodeReader): CausalRelation {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: return new CausalRelation('Equal', {});
      case 1: return new CausalRelation('StrictDescends', {});
      case 2: return new CausalRelation('StrictAscends', {});
      case 3: {
        const meet = Clock.decode(reader);
        const subject = Clock.decode(reader);
        const other = Clock.decode(reader);
        return new CausalRelation('DivergedSince', { meet, subject, other });
      }
      case 4: {
        const gca = reader.readOption(r => Clock.decode(r));
        const subjectRoot = EventId.decode(reader);
        const otherRoot = EventId.decode(reader);
        return new CausalRelation('Disjoint', { gca, subjectRoot, otherRoot });
      }
      case 5: {
        const subject = Clock.decode(reader);
        const other = Clock.decode(reader);
        return new CausalRelation('BudgetExceeded', { subject, other });
      }
      default:
        throw new Error(`Unknown CausalRelation variant: ${variant}`);
    }
  }
}

// ─── CausalAssertion ────────────────────────────────────────────────────────

export class CausalAssertion extends Struct {
  readonly entityId: EntityId;
  readonly subject: Clock;
  readonly other: Clock;
  readonly relation: CausalRelation;

  constructor(entityId: EntityId, subject: Clock, other: Clock, relation: CausalRelation) {
    super();
    this.entityId = entityId;
    this.subject = subject;
    this.other = other;
    this.relation = relation;
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.subject.encode(writer);
    this.other.encode(writer);
    this.relation.encode(writer);
  }

  static decode(reader: BincodeReader): CausalAssertion {
    const entityId = EntityId.decode(reader);
    const subject = Clock.decode(reader);
    const other = Clock.decode(reader);
    const relation = CausalRelation.decode(reader);
    return new CausalAssertion(entityId, subject, other, relation);
  }
}

// ─── CausalAssertionFragment ────────────────────────────────────────────────

/// Wire-minimal lineage attestation
export class CausalAssertionFragment extends Struct {
  readonly relation: CausalRelation;
  readonly attestations: AttestationSet;

  constructor(relation: CausalRelation, attestations: AttestationSet) {
    super();
    this.relation = relation;
    this.attestations = attestations;
  }

  encode(writer: BincodeWriter): void {
    this.relation.encode(writer);
    this.attestations.encode(writer);
  }

  static decode(reader: BincodeReader): CausalAssertionFragment {
    const relation = CausalRelation.decode(reader);
    const attestations = AttestationSet.decode(reader);
    return new CausalAssertionFragment(relation, attestations);
  }
}

// ─── DeltaContent ───────────────────────────────────────────────────────────

type DeltaContentV = {
  StateSnapshot: { state: StateFragment };
  EventBridge: { events: EventFragment[] };
  StateAndRelation: { state: StateFragment; relation: CausalAssertionFragment };
};

export class DeltaContent extends Enum<DeltaContentV> {
  encode(writer: BincodeWriter): void {
    this.match({
      StateSnapshot: (v) => {
        writer.writeVariant(0);
        v.state.encode(writer);
      },
      EventBridge: (v) => {
        writer.writeVariant(1);
        writer.writeVec(v.events, (w, e) => e.encode(w));
      },
      StateAndRelation: (v) => {
        writer.writeVariant(2);
        v.state.encode(writer);
        v.relation.encode(writer);
      },
    });
  }

  static decode(reader: BincodeReader): DeltaContent {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const state = StateFragment.decode(reader);
        return new DeltaContent('StateSnapshot', { state });
      }
      case 1: {
        const events = reader.readVec(r => EventFragment.decode(r));
        return new DeltaContent('EventBridge', { events });
      }
      case 2: {
        const state = StateFragment.decode(reader);
        const relation = CausalAssertionFragment.decode(reader);
        return new DeltaContent('StateAndRelation', { state, relation });
      }
      default:
        throw new Error(`Unknown DeltaContent variant: ${variant}`);
    }
  }
}

// ─── EntityDelta ────────────────────────────────────────────────────────────

/// Entity initialization data returned in QuerySubscribed and Fetch
export class EntityDelta extends Struct {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly content: DeltaContent;

  constructor(entityId: EntityId, collection: CollectionId, content: DeltaContent) {
    super();
    this.entityId = entityId;
    this.collection = collection;
    this.content = content;
  }

  // impl Display for EntityDelta
  toString(): string {
    return this.content.match({
      StateSnapshot: (v) => `EntityDelta ${this.entityId}: StateSnapshot(${v.state})`,
      EventBridge: (v) => `EntityDelta ${this.entityId}: EventBridge(${v.events.length} events)`,
      StateAndRelation: (v) => `EntityDelta ${this.entityId}: StateAndRelation(${v.state})`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.collection.encode(writer);
    this.content.encode(writer);
  }

  static decode(reader: BincodeReader): EntityDelta {
    const entityId = EntityId.decode(reader);
    const collection = CollectionId.decode(reader);
    const content = DeltaContent.decode(reader);
    return new EntityDelta(entityId, collection, content);
  }
}

// ─── NodeRequestBody ────────────────────────────────────────────────────────

// Divergence: selection field stored as Uint8Array (raw bincode bytes) until @ankurah/ankql
// provides Selection.encode/decode. See ast::Selection in ankql.

// Divergence: SubscribeEntity (variant 2) not in Rust — TS-ahead variant for entity-level subscribe.
// This shifts GetEvents/Fetch/SubscribeQuery variant numbers vs Rust wire format. [E4]
type NodeRequestBodyV = {
  CommitTransaction: { id: TransactionId; events: Attested<Event>[] };
  Get: { collection: CollectionId; ids: EntityId[] };
  SubscribeEntity: { collection: CollectionId; ids: EntityId[]; knownEntities: KnownEntity[] };
  GetEvents: { collection: CollectionId; eventIds: EventId[] };
  Fetch: { collection: CollectionId; selection: Uint8Array; knownMatches: KnownEntity[] };
  SubscribeQuery: { queryId: QueryId; collection: CollectionId; selection: Uint8Array; version: number; knownMatches: KnownEntity[] };
};

export class NodeRequestBody extends Enum<NodeRequestBodyV> {
  // impl Display for NodeRequestBody
  toString(): string {
    return this.match({
      CommitTransaction: (v) => `CommitTransaction ${v.id} [${v.events.length} events]`,
      Get: (v) => `Get ${v.collection} ${v.ids.map(id => id.toBase64Short()).join(', ')}`,
      SubscribeEntity: (v) => `SubscribeEntity ${v.collection} ids:${v.ids.length} known:${v.knownEntities.length}`,
      GetEvents: (v) => `GetEvents ${v.collection} ${v.eventIds.map(id => id.toBase64Short()).join(', ')}`,
      Fetch: (v) => `Fetch ${v.collection} known:${v.knownMatches.length}`,
      SubscribeQuery: (v) => `Subscribe ${v.queryId} ${v.collection} v${v.version} known:${v.knownMatches.length}`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      CommitTransaction: (v) => {
        writer.writeVariant(0);
        v.id.encode(writer);
        writer.writeVec(v.events, (w, e) => {
          e.encode(w, (w2, event) => event.encode(w2));
        });
      },
      Get: (v) => {
        writer.writeVariant(1);
        v.collection.encode(writer);
        writer.writeVec(v.ids, (w, id) => id.encode(w));
      },
      SubscribeEntity: (v) => {
        writer.writeVariant(2);
        v.collection.encode(writer);
        writer.writeVec(v.ids, (w, id) => id.encode(w));
        writer.writeVec(v.knownEntities, (w, ke) => ke.encode(w));
      },
      GetEvents: (v) => {
        writer.writeVariant(3);
        v.collection.encode(writer);
        writer.writeVec(v.eventIds, (w, id) => id.encode(w));
      },
      Fetch: (v) => {
        writer.writeVariant(4);
        v.collection.encode(writer);
        writer.writeRawBytes(v.selection);
        writer.writeVec(v.knownMatches, (w, km) => km.encode(w));
      },
      SubscribeQuery: (v) => {
        writer.writeVariant(5);
        v.queryId.encode(writer);
        v.collection.encode(writer);
        writer.writeRawBytes(v.selection);
        writer.writeU32(v.version);
        writer.writeVec(v.knownMatches, (w, km) => km.encode(w));
      },
    });
  }

  static decode(reader: BincodeReader): NodeRequestBody {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const id = TransactionId.decode(reader);
        const events = reader.readVec(r => Attested.decode(r, r2 => Event.decode(r2)));
        return new NodeRequestBody('CommitTransaction', { id, events });
      }
      case 1: {
        const collection = CollectionId.decode(reader);
        const ids = reader.readVec(r => EntityId.decode(r));
        return new NodeRequestBody('Get', { collection, ids });
      }
      case 2: {
        const collection = CollectionId.decode(reader);
        const ids = reader.readVec(r => EntityId.decode(r));
        const knownEntities = reader.readVec(r => KnownEntity.decode(r));
        return new NodeRequestBody('SubscribeEntity', { collection, ids, knownEntities });
      }
      case 3: {
        const collection = CollectionId.decode(reader);
        const eventIds = reader.readVec(r => EventId.decode(r));
        return new NodeRequestBody('GetEvents', { collection, eventIds });
      }
      case 4: {
        const collection = CollectionId.decode(reader);
        // Divergence: Selection stored as opaque bytes until @ankurah/ankql provides encode/decode
        const selection = decodeOpaqueSelection(reader);
        const knownMatches = reader.readVec(r => KnownEntity.decode(r));
        return new NodeRequestBody('Fetch', { collection, selection, knownMatches });
      }
      case 5: {
        const queryId = QueryId.decode(reader);
        const collection = CollectionId.decode(reader);
        const selection = decodeOpaqueSelection(reader);
        const version = reader.readU32();
        const knownMatches = reader.readVec(r => KnownEntity.decode(r));
        return new NodeRequestBody('SubscribeQuery', { queryId, collection, selection, version, knownMatches });
      }
      default:
        throw new Error(`Unknown NodeRequestBody variant: ${variant}`);
    }
  }
}

/**
 * Placeholder: Selection decode requires @ankurah/ankql Selection.decode().
 * TODO: Replace with proper Selection codec when available.
 */
function decodeOpaqueSelection(_reader: BincodeReader): Uint8Array {
  throw new Error('Selection decode not yet implemented — requires @ankurah/ankql Selection.encode/decode');
}

// ─── NodeResponse ───────────────────────────────────────────────────────────

/// A response from one node to another
export class NodeResponse extends Struct {
  readonly requestId: RequestId;
  readonly from: EntityId;
  readonly to: EntityId;
  readonly body: NodeResponseBody;

  constructor(requestId: RequestId, from: EntityId, to: EntityId, body: NodeResponseBody) {
    super();
    this.requestId = requestId;
    this.from = from;
    this.to = to;
    this.body = body;
  }

  // impl Display for NodeResponse
  toString(): string {
    return `Response(${this.requestId}) ${this.from}->${this.to} ${this.body}`;
  }

  encode(writer: BincodeWriter): void {
    this.requestId.encode(writer);
    this.from.encode(writer);
    this.to.encode(writer);
    this.body.encode(writer);
  }

  static decode(reader: BincodeReader): NodeResponse {
    const requestId = RequestId.decode(reader);
    const from = EntityId.decode(reader);
    const to = EntityId.decode(reader);
    const body = NodeResponseBody.decode(reader);
    return new NodeResponse(requestId, from, to, body);
  }
}

// ─── NodeResponseBody ───────────────────────────────────────────────────────

// Divergence: EntitiesSubscribed (variant 3) not in Rust — TS-ahead variant for entity-level subscribe response.
// This shifts GetEvents/QuerySubscribed/Success/Error variant numbers vs Rust wire format. [E4]
type NodeResponseBodyV = {
  CommitComplete: { id: TransactionId };
  Fetch: { deltas: EntityDelta[] };
  Get: { states: Attested<EntityState>[] };
  EntitiesSubscribed: { deltas: EntityDelta[] };
  GetEvents: { events: Attested<Event>[] };
  QuerySubscribed: { queryId: QueryId; deltas: EntityDelta[] };
  Success: {};
  Error: { message: string };
};

export class NodeResponseBody extends Enum<NodeResponseBodyV> {
  // impl Display for NodeResponseBody
  toString(): string {
    return this.match({
      CommitComplete: (v) => `CommitComplete ${v.id}`,
      Fetch: (v) => `Fetch [${v.deltas.length}]`,
      Get: (v) => `Get [${v.states.length}]`,
      EntitiesSubscribed: (v) => `EntitiesSubscribed [${v.deltas.length}]`,
      GetEvents: (v) => `GetEvents [${v.events.length}]`,
      QuerySubscribed: (v) => `Subscribed ${v.queryId} initial:${v.deltas.length}`,
      Success: () => 'Success',
      Error: (v) => `Error: ${v.message}`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      CommitComplete: (v) => {
        writer.writeVariant(0);
        v.id.encode(writer);
      },
      Fetch: (v) => {
        writer.writeVariant(1);
        writer.writeVec(v.deltas, (w, d) => d.encode(w));
      },
      Get: (v) => {
        writer.writeVariant(2);
        writer.writeVec(v.states, (w, s) => {
          s.encode(w, (w2, es) => es.encode(w2));
        });
      },
      EntitiesSubscribed: (v) => {
        writer.writeVariant(3);
        writer.writeVec(v.deltas, (w, d) => d.encode(w));
      },
      GetEvents: (v) => {
        writer.writeVariant(4);
        writer.writeVec(v.events, (w, e) => {
          e.encode(w, (w2, ev) => ev.encode(w2));
        });
      },
      QuerySubscribed: (v) => {
        writer.writeVariant(5);
        v.queryId.encode(writer);
        writer.writeVec(v.deltas, (w, d) => d.encode(w));
      },
      Success: () => {
        writer.writeVariant(6);
      },
      Error: (v) => {
        writer.writeVariant(7);
        writer.writeString(v.message);
      },
    });
  }

  static decode(reader: BincodeReader): NodeResponseBody {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const id = TransactionId.decode(reader);
        return new NodeResponseBody('CommitComplete', { id });
      }
      case 1: {
        const deltas = reader.readVec(r => EntityDelta.decode(r));
        return new NodeResponseBody('Fetch', { deltas });
      }
      case 2: {
        const states = reader.readVec(r => Attested.decode(r, r2 => EntityState.decode(r2)));
        return new NodeResponseBody('Get', { states });
      }
      case 3: {
        const deltas = reader.readVec(r => EntityDelta.decode(r));
        return new NodeResponseBody('EntitiesSubscribed', { deltas });
      }
      case 4: {
        const events = reader.readVec(r => Attested.decode(r, r2 => Event.decode(r2)));
        return new NodeResponseBody('GetEvents', { events });
      }
      case 5: {
        const queryId = QueryId.decode(reader);
        const deltas = reader.readVec(r => EntityDelta.decode(r));
        return new NodeResponseBody('QuerySubscribed', { queryId, deltas });
      }
      case 6:
        return new NodeResponseBody('Success', {});
      case 7: {
        const message = reader.readString();
        return new NodeResponseBody('Error', { message });
      }
      default:
        throw new Error(`Unknown NodeResponseBody variant: ${variant}`);
    }
  }
}
