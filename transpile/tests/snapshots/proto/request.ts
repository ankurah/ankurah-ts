// MIRRORS: ankurah/proto/src/request.rs
import { Struct, Enum } from '@ankurah/base';
import { RequestId } from './id.provided';
import { BincodeReader, BincodeWriter } from './codec';
import { AttestationSet, Attested } from './auth';
import { Clock } from './clock';
import { CollectionId } from './collection';
import { EntityState, Event, EventFragment, EventId, StateFragment } from './data';
import { EntityId } from './id';
import { QueryId } from './subscription';
import { TransactionId } from './transaction';
import { Selection } from '@ankurah/ankql';
export { RequestId };

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

  toString(): string {
    return `Request ${this.id} from ${this.from}->${this.to}: ${this.body}`;
  }

  debug(): string {
    return `NodeRequest { id: ${this.id}, to: ${this.to}, from: ${this.from}, body: ${this.body.debug()} }`;
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

export class KnownEntity extends Struct {
  readonly entityId: EntityId;
  readonly head: Clock;

  constructor(entityId: EntityId, head: Clock) {
    super();
    this.entityId = entityId;
    this.head = head;
  }

  clone(): KnownEntity {
    return new KnownEntity(this.entityId.clone(), this.head.clone());
  }

  debug(): string {
    return `KnownEntity { entityId: ${this.entityId}, head: ${this.head} }`;
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

  clone(): CausalAssertion {
    return new CausalAssertion(this.entityId.clone(), this.subject.clone(), this.other.clone(), this.relation.clone());
  }

  debug(): string {
    return `CausalAssertion { entityId: ${this.entityId}, subject: ${this.subject}, other: ${this.other}, relation: ${this.relation.debug()} }`;
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

export class CausalAssertionFragment extends Struct {
  readonly relation: CausalRelation;
  readonly attestations: AttestationSet;

  constructor(relation: CausalRelation, attestations: AttestationSet) {
    super();
    this.relation = relation;
    this.attestations = attestations;
  }

  clone(): CausalAssertionFragment {
    return new CausalAssertionFragment(this.relation.clone(), this.attestations.clone());
  }

  debug(): string {
    return `CausalAssertionFragment { relation: ${this.relation.debug()}, attestations: ${this.attestations.debug()} }`;
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

  toString(): string {
    return `[EntityDelta]`;
  }

  clone(): EntityDelta {
    return new EntityDelta(this.entityId.clone(), this.collection.clone(), this.content.clone());
  }

  debug(): string {
    return `EntityDelta { entityId: ${this.entityId}, collection: ${this.collection.debug()}, content: ${this.content.debug()} }`;
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

  toString(): string {
    return `Response(${this.requestId}) ${this.from}->${this.to} ${this.body}`;
  }

  debug(): string {
    return `NodeResponse { requestId: ${this.requestId}, from: ${this.from}, to: ${this.to}, body: ${this.body.debug()} }`;
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

export type CausalRelationV = {
  Equal: {};
  StrictDescends: {};
  StrictAscends: {};
  DivergedSince: { meet: Clock; subject: Clock; other: Clock };
  Disjoint: { gca: Clock | null; subjectRoot: EventId; otherRoot: EventId };
  BudgetExceeded: { subject: Clock; other: Clock };
};

export class CausalRelation extends Enum<CausalRelationV> {

  clone(): CausalRelation {
    return this.match({
      Equal: () => new CausalRelation('Equal', {}),
      StrictDescends: () => new CausalRelation('StrictDescends', {}),
      StrictAscends: () => new CausalRelation('StrictAscends', {}),
      DivergedSince: (v) => new CausalRelation('DivergedSince', { meet: v.meet.clone(), subject: v.subject.clone(), other: v.other.clone() }),
      Disjoint: (v) => new CausalRelation('Disjoint', { gca: v.gca?.clone() ?? null, subjectRoot: v.subjectRoot.clone(), otherRoot: v.otherRoot.clone() }),
      BudgetExceeded: (v) => new CausalRelation('BudgetExceeded', { subject: v.subject.clone(), other: v.other.clone() }),
    });
  }

  debug(): string {
    return this.match({
      Equal: () => 'Equal',
      StrictDescends: () => 'StrictDescends',
      StrictAscends: () => 'StrictAscends',
      DivergedSince: (v) => `DivergedSince { meet: ${v.meet}, subject: ${v.subject}, other: ${v.other} }`,
      Disjoint: (v) => `Disjoint { gca: ${v.gca}, subjectRoot: ${v.subjectRoot}, otherRoot: ${v.otherRoot} }`,
      BudgetExceeded: (v) => `BudgetExceeded { subject: ${v.subject}, other: ${v.other} }`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Equal: (v) => {
        writer.writeVariant(0);
      },
      StrictDescends: (v) => {
        writer.writeVariant(1);
      },
      StrictAscends: (v) => {
        writer.writeVariant(2);
      },
      DivergedSince: (v) => {
        writer.writeVariant(3);
        v.meet.encode(writer);
        v.subject.encode(writer);
        v.other.encode(writer);
      },
      Disjoint: (v) => {
        writer.writeVariant(4);
        writer.writeOption(v.gca, (w, v) => v.encode(w));
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
      case 0: {
        return new CausalRelation('Equal', {});
      }
      case 1: {
        return new CausalRelation('StrictDescends', {});
      }
      case 2: {
        return new CausalRelation('StrictAscends', {});
      }
      case 3: {
        const meet = Clock.decode(reader);
        const subject = Clock.decode(reader);
        const other = Clock.decode(reader);
        return new CausalRelation('DivergedSince', { meet, subject, other });
      }
      case 4: {
        const gca = reader.readOption((r) => Clock.decode(r));
        const subjectRoot = EventId.decode(reader);
        const otherRoot = EventId.decode(reader);
        return new CausalRelation('Disjoint', { gca, subjectRoot, otherRoot });
      }
      case 5: {
        const subject = Clock.decode(reader);
        const other = Clock.decode(reader);
        return new CausalRelation('BudgetExceeded', { subject, other });
      }
      default: throw new Error(`Unknown CausalRelation variant: ${variant}`);
    }
  }
}

export type DeltaContentV = {
  StateSnapshot: { state: StateFragment };
  EventBridge: { events: EventFragment[] };
  StateAndRelation: { state: StateFragment; relation: CausalAssertionFragment };
};

export class DeltaContent extends Enum<DeltaContentV> {

  clone(): DeltaContent {
    return this.match({
      StateSnapshot: (v) => new DeltaContent('StateSnapshot', { state: v.state.clone() }),
      EventBridge: (v) => new DeltaContent('EventBridge', { events: v.events.map(e => e.clone()) }),
      StateAndRelation: (v) => new DeltaContent('StateAndRelation', { state: v.state.clone(), relation: v.relation.clone() }),
    });
  }

  debug(): string {
    return this.match({
      StateSnapshot: (v) => `StateSnapshot { state: ${v.state.debug()} }`,
      EventBridge: (v) => `EventBridge { events: ${`[${Array.from(v.events).map((e) => e.debug()).join(', ')}]`} }`,
      StateAndRelation: (v) => `StateAndRelation { state: ${v.state.debug()}, relation: ${v.relation.debug()} }`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      StateSnapshot: (v) => {
        writer.writeVariant(0);
        v.state.encode(writer);
      },
      EventBridge: (v) => {
        writer.writeVariant(1);
        writer.writeVec(v.events, (w, item) => item.encode(w));
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
        const events = reader.readVec((r) => EventFragment.decode(r));
        return new DeltaContent('EventBridge', { events });
      }
      case 2: {
        const state = StateFragment.decode(reader);
        const relation = CausalAssertionFragment.decode(reader);
        return new DeltaContent('StateAndRelation', { state, relation });
      }
      default: throw new Error(`Unknown DeltaContent variant: ${variant}`);
    }
  }
}

export type NodeRequestBodyV = {
  CommitTransaction: { id: TransactionId; events: Attested<Event>[] };
  Get: { collection: CollectionId; ids: EntityId[] };
  GetEvents: { collection: CollectionId; eventIds: EventId[] };
  Fetch: { collection: CollectionId; selection: Selection; knownMatches: KnownEntity[] };
  SubscribeQuery: { queryId: QueryId; collection: CollectionId; selection: Selection; version: number; knownMatches: KnownEntity[] };
};

export class NodeRequestBody extends Enum<NodeRequestBodyV> {

  toString(): string {
    return this.match({
      CommitTransaction: (v) => {
        const id = v.id;
        const events = v.events;
        return `CommitTransaction ${id} [${[...events].map((e) => `${e}`).join(', ')}]`;
      },
      Get: (v) => {
        const collection = v.collection;
        const ids = v.ids;
        return `Get ${collection} ${[...ids].map((id) => id.toBase64Short()).join(', ')}`;
      },
      GetEvents: (v) => {
        const collection = v.collection;
        const eventIds = v.eventIds;
        return `GetEvents ${collection} ${[...eventIds].map((id) => id.toBase64Short()).join(', ')}`;
      },
      Fetch: (v) => {
        const collection = v.collection;
        const query = v.selection;
        const knownMatches = v.knownMatches;
        return `Fetch ${collection} ${query} known:${knownMatches.length}`;
      },
      SubscribeQuery: (v) => {
        const queryId = v.queryId;
        const collection = v.collection;
        const query = v.selection;
        const version = v.version;
        const knownMatches = v.knownMatches;
        return `Subscribe ${queryId} ${collection} ${query} v${version} known:${knownMatches.length}`;
      },
    });
  }

  debug(): string {
    return this.match({
      CommitTransaction: (v) => `CommitTransaction { id: ${v.id}, events: ${v.events} }`,
      Get: (v) => `Get { collection: ${v.collection.debug()}, ids: ${v.ids} }`,
      GetEvents: (v) => `GetEvents { collection: ${v.collection.debug()}, eventIds: ${v.eventIds} }`,
      Fetch: (v) => `Fetch { collection: ${v.collection.debug()}, selection: ${v.selection}, knownMatches: ${`[${Array.from(v.knownMatches).map((e) => e.debug()).join(', ')}]`} }`,
      SubscribeQuery: (v) => `SubscribeQuery { queryId: ${v.queryId}, collection: ${v.collection.debug()}, selection: ${v.selection}, version: ${String(v.version)}, knownMatches: ${`[${Array.from(v.knownMatches).map((e) => e.debug()).join(', ')}]`} }`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      CommitTransaction: (v) => {
        writer.writeVariant(0);
        v.id.encode(writer);
        writer.writeVec(v.events, (w, item) => item.encode(w, (w2: BincodeWriter, p: Event) => p.encode(w2)));
      },
      Get: (v) => {
        writer.writeVariant(1);
        v.collection.encode(writer);
        writer.writeVec(v.ids, (w, item) => item.encode(w));
      },
      GetEvents: (v) => {
        writer.writeVariant(2);
        v.collection.encode(writer);
        writer.writeVec(v.eventIds, (w, item) => item.encode(w));
      },
      Fetch: (v) => {
        writer.writeVariant(3);
        v.collection.encode(writer);
        v.selection.encode(writer);
        writer.writeVec(v.knownMatches, (w, item) => item.encode(w));
      },
      SubscribeQuery: (v) => {
        writer.writeVariant(4);
        v.queryId.encode(writer);
        v.collection.encode(writer);
        v.selection.encode(writer);
        writer.writeU32(v.version);
        writer.writeVec(v.knownMatches, (w, item) => item.encode(w));
      },
    });
  }

  static decode(reader: BincodeReader): NodeRequestBody {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const id = TransactionId.decode(reader);
        const events = reader.readVec((r) => Attested.decode(r, (r2: BincodeReader) => Event.decode(r2)));
        return new NodeRequestBody('CommitTransaction', { id, events });
      }
      case 1: {
        const collection = CollectionId.decode(reader);
        const ids = reader.readVec((r) => EntityId.decode(r));
        return new NodeRequestBody('Get', { collection, ids });
      }
      case 2: {
        const collection = CollectionId.decode(reader);
        const eventIds = reader.readVec((r) => EventId.decode(r));
        return new NodeRequestBody('GetEvents', { collection, eventIds });
      }
      case 3: {
        const collection = CollectionId.decode(reader);
        const selection = Selection.decode(reader);
        const knownMatches = reader.readVec((r) => KnownEntity.decode(r));
        return new NodeRequestBody('Fetch', { collection, selection, knownMatches });
      }
      case 4: {
        const queryId = QueryId.decode(reader);
        const collection = CollectionId.decode(reader);
        const selection = Selection.decode(reader);
        const version = reader.readU32();
        const knownMatches = reader.readVec((r) => KnownEntity.decode(r));
        return new NodeRequestBody('SubscribeQuery', { queryId, collection, selection, version, knownMatches });
      }
      default: throw new Error(`Unknown NodeRequestBody variant: ${variant}`);
    }
  }
}

export type NodeResponseBodyV = {
  CommitComplete: { id: TransactionId };
  Fetch: { _0: EntityDelta[] };
  Get: { _0: Attested<EntityState>[] };
  GetEvents: { _0: Attested<Event>[] };
  QuerySubscribed: { queryId: QueryId; deltas: EntityDelta[] };
  Success: {};
  Error: { _0: string };
};

export class NodeResponseBody extends Enum<NodeResponseBodyV> {

  toString(): string {
    return this.match({
      CommitComplete: (v) => {
        const id = v.id;
        return `CommitComplete ${id}`;
      },
      Fetch: (v) => {
        const deltas = v._0;
        return `Fetch [${deltas.length}]`;
      },
      Get: (v) => {
        const states = v._0;
        return `Get [${[...states].map((s) => s.toString()).join(', ')}]`;
      },
      GetEvents: (v) => {
        const events = v._0;
        return `GetEvents [${[...events].map((e) => e.payload.toString()).join(', ')}]`;
      },
      QuerySubscribed: (v) => {
        const queryId = v.queryId;
        const initial = v.deltas;
        return `Subscribed ${queryId} initial:${initial.length}`;
      },
      Success: () => 'Success',
      Error: (v) => {
        const e = v._0;
        return `Error: ${e}`;
      },
    });
  }

  debug(): string {
    return this.match({
      CommitComplete: (v) => `CommitComplete { id: ${v.id} }`,
      Fetch: (v) => `Fetch(${`[${Array.from(v._0).map((e) => e.debug()).join(', ')}]`})`,
      Get: (v) => `Get(${v._0})`,
      GetEvents: (v) => `GetEvents(${v._0})`,
      QuerySubscribed: (v) => `QuerySubscribed { queryId: ${v.queryId}, deltas: ${`[${Array.from(v.deltas).map((e) => e.debug()).join(', ')}]`} }`,
      Success: () => 'Success',
      Error: (v) => `Error(${JSON.stringify(v._0)})`,
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
        writer.writeVec(v._0, (w, item) => item.encode(w));
      },
      Get: (v) => {
        writer.writeVariant(2);
        writer.writeVec(v._0, (w, item) => item.encode(w, (w2: BincodeWriter, p: EntityState) => p.encode(w2)));
      },
      GetEvents: (v) => {
        writer.writeVariant(3);
        writer.writeVec(v._0, (w, item) => item.encode(w, (w2: BincodeWriter, p: Event) => p.encode(w2)));
      },
      QuerySubscribed: (v) => {
        writer.writeVariant(4);
        v.queryId.encode(writer);
        writer.writeVec(v.deltas, (w, item) => item.encode(w));
      },
      Success: (v) => {
        writer.writeVariant(5);
      },
      Error: (v) => {
        writer.writeVariant(6);
        writer.writeString(v._0);
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
        const _0 = reader.readVec((r) => EntityDelta.decode(r));
        return new NodeResponseBody('Fetch', { _0 });
      }
      case 2: {
        const _0 = reader.readVec((r) => Attested.decode(r, (r2: BincodeReader) => EntityState.decode(r2)));
        return new NodeResponseBody('Get', { _0 });
      }
      case 3: {
        const _0 = reader.readVec((r) => Attested.decode(r, (r2: BincodeReader) => Event.decode(r2)));
        return new NodeResponseBody('GetEvents', { _0 });
      }
      case 4: {
        const queryId = QueryId.decode(reader);
        const deltas = reader.readVec((r) => EntityDelta.decode(r));
        return new NodeResponseBody('QuerySubscribed', { queryId, deltas });
      }
      case 5: {
        return new NodeResponseBody('Success', {});
      }
      case 6: {
        const _0 = reader.readString();
        return new NodeResponseBody('Error', { _0 });
      }
      default: throw new Error(`Unknown NodeResponseBody variant: ${variant}`);
    }
  }
}

