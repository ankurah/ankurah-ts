// MIRRORS: ankurah/proto/src/request.rs
import { Struct, Enum, Result, JsonError } from '@ankurah/base';
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

  toJSON(): unknown {
    return {
      'id': this.id,
      'to': this.to,
      'from': this.from,
      'body': this.body,
    };
  }

  static fromJson(value: unknown): Result<NodeRequest, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      const id = ((v: unknown) => _take(RequestId.fromJson(v)))(o['id']);
      const to = ((v: unknown) => _take(EntityId.fromJson(v)))(o['to']);
      const from = ((v: unknown) => _take(EntityId.fromJson(v)))(o['from']);
      const body = ((v: unknown) => _take(NodeRequestBody.fromJson(v)))(o['body']);
      return Result.Ok(new NodeRequest(id, to, from, body));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
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

  toJSON(): unknown {
    return {
      'entity_id': this.entityId,
      'head': this.head,
    };
  }

  static fromJson(value: unknown): Result<KnownEntity, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      const entityId = ((v: unknown) => _take(EntityId.fromJson(v)))(o['entity_id']);
      const head = ((v: unknown) => _take(Clock.fromJson(v)))(o['head']);
      return Result.Ok(new KnownEntity(entityId, head));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
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

  toJSON(): unknown {
    return {
      'entity_id': this.entityId,
      'subject': this.subject,
      'other': this.other,
      'relation': this.relation,
    };
  }

  static fromJson(value: unknown): Result<CausalAssertion, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      const entityId = ((v: unknown) => _take(EntityId.fromJson(v)))(o['entity_id']);
      const subject = ((v: unknown) => _take(Clock.fromJson(v)))(o['subject']);
      const other = ((v: unknown) => _take(Clock.fromJson(v)))(o['other']);
      const relation = ((v: unknown) => _take(CausalRelation.fromJson(v)))(o['relation']);
      return Result.Ok(new CausalAssertion(entityId, subject, other, relation));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
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

  toJSON(): unknown {
    return {
      'relation': this.relation,
      'attestations': this.attestations,
    };
  }

  static fromJson(value: unknown): Result<CausalAssertionFragment, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      const relation = ((v: unknown) => _take(CausalRelation.fromJson(v)))(o['relation']);
      const attestations = ((v: unknown) => _take(AttestationSet.fromJson(v)))(o['attestations']);
      return Result.Ok(new CausalAssertionFragment(relation, attestations));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
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

  toJSON(): unknown {
    return {
      'entity_id': this.entityId,
      'collection': this.collection,
      'content': this.content,
    };
  }

  static fromJson(value: unknown): Result<EntityDelta, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      const entityId = ((v: unknown) => _take(EntityId.fromJson(v)))(o['entity_id']);
      const collection = ((v: unknown) => _take(CollectionId.fromJson(v)))(o['collection']);
      const content = ((v: unknown) => _take(DeltaContent.fromJson(v)))(o['content']);
      return Result.Ok(new EntityDelta(entityId, collection, content));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
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

  toJSON(): unknown {
    return {
      'request_id': this.requestId,
      'from': this.from,
      'to': this.to,
      'body': this.body,
    };
  }

  static fromJson(value: unknown): Result<NodeResponse, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      const requestId = ((v: unknown) => _take(RequestId.fromJson(v)))(o['request_id']);
      const from = ((v: unknown) => _take(EntityId.fromJson(v)))(o['from']);
      const to = ((v: unknown) => _take(EntityId.fromJson(v)))(o['to']);
      const body = ((v: unknown) => _take(NodeResponseBody.fromJson(v)))(o['body']);
      return Result.Ok(new NodeResponse(requestId, from, to, body));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
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

  toJSON(): unknown {
    return this.match<unknown>({
      Equal: () => 'Equal',
      StrictDescends: () => 'StrictDescends',
      StrictAscends: () => 'StrictAscends',
      DivergedSince: (v) => ({ 'DivergedSince': { 'meet': v.meet, 'subject': v.subject, 'other': v.other } }),
      Disjoint: (v) => ({ 'Disjoint': { 'gca': v.gca, 'subject_root': v.subjectRoot, 'other_root': v.otherRoot } }),
      BudgetExceeded: (v) => ({ 'BudgetExceeded': { 'subject': v.subject, 'other': v.other } }),
    });
  }

  static fromJson(value: unknown): Result<CausalRelation, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      if (typeof value === 'string') {
        switch (value) {
          case 'Equal': return Result.Ok(new CausalRelation('Equal', {}));
          case 'StrictDescends': return Result.Ok(new CausalRelation('StrictDescends', {}));
          case 'StrictAscends': return Result.Ok(new CausalRelation('StrictAscends', {}));
        }
      }
      const o = value as Record<string, unknown>;
      if ('DivergedSince' in o) {
        const p = o['DivergedSince'];
        return Result.Ok(new CausalRelation('DivergedSince', { meet: ((v: unknown) => _take(Clock.fromJson(v)))((p as Record<string, unknown>)['meet']), subject: ((v: unknown) => _take(Clock.fromJson(v)))((p as Record<string, unknown>)['subject']), other: ((v: unknown) => _take(Clock.fromJson(v)))((p as Record<string, unknown>)['other']) }));
      }
      if ('Disjoint' in o) {
        const p = o['Disjoint'];
        return Result.Ok(new CausalRelation('Disjoint', { gca: ((v: unknown) => (v == null ? null : ((v) => _take(Clock.fromJson(v)))(v)))((p as Record<string, unknown>)['gca']), subjectRoot: ((v: unknown) => _take(EventId.fromJson(v)))((p as Record<string, unknown>)['subject_root']), otherRoot: ((v: unknown) => _take(EventId.fromJson(v)))((p as Record<string, unknown>)['other_root']) }));
      }
      if ('BudgetExceeded' in o) {
        const p = o['BudgetExceeded'];
        return Result.Ok(new CausalRelation('BudgetExceeded', { subject: ((v: unknown) => _take(Clock.fromJson(v)))((p as Record<string, unknown>)['subject']), other: ((v: unknown) => _take(Clock.fromJson(v)))((p as Record<string, unknown>)['other']) }));
      }
      return Result.Err(JsonError.custom('no variant of `CausalRelation` matches this JSON'));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
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

  toJSON(): unknown {
    return this.match<unknown>({
      StateSnapshot: (v) => ({ 'StateSnapshot': { 'state': v.state } }),
      EventBridge: (v) => ({ 'EventBridge': { 'events': v.events } }),
      StateAndRelation: (v) => ({ 'StateAndRelation': { 'state': v.state, 'relation': v.relation } }),
    });
  }

  static fromJson(value: unknown): Result<DeltaContent, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      if ('StateSnapshot' in o) {
        const p = o['StateSnapshot'];
        return Result.Ok(new DeltaContent('StateSnapshot', { state: ((v: unknown) => _take(StateFragment.fromJson(v)))((p as Record<string, unknown>)['state']) }));
      }
      if ('EventBridge' in o) {
        const p = o['EventBridge'];
        return Result.Ok(new DeltaContent('EventBridge', { events: ((v: unknown) => (v as unknown[]).map((v) => _take(EventFragment.fromJson(v))))((p as Record<string, unknown>)['events']) }));
      }
      if ('StateAndRelation' in o) {
        const p = o['StateAndRelation'];
        return Result.Ok(new DeltaContent('StateAndRelation', { state: ((v: unknown) => _take(StateFragment.fromJson(v)))((p as Record<string, unknown>)['state']), relation: ((v: unknown) => _take(CausalAssertionFragment.fromJson(v)))((p as Record<string, unknown>)['relation']) }));
      }
      return Result.Err(JsonError.custom('no variant of `DeltaContent` matches this JSON'));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
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
      Fetch: (v) => `Fetch { collection: ${v.collection.debug()}, selection: ${v.selection.debug()}, knownMatches: ${`[${Array.from(v.knownMatches).map((e) => e.debug()).join(', ')}]`} }`,
      SubscribeQuery: (v) => `SubscribeQuery { queryId: ${v.queryId}, collection: ${v.collection.debug()}, selection: ${v.selection.debug()}, version: ${String(v.version)}, knownMatches: ${`[${Array.from(v.knownMatches).map((e) => e.debug()).join(', ')}]`} }`,
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

  toJSON(): unknown {
    return this.match<unknown>({
      CommitTransaction: (v) => ({ 'CommitTransaction': { 'id': v.id, 'events': v.events } }),
      Get: (v) => ({ 'Get': { 'collection': v.collection, 'ids': v.ids } }),
      GetEvents: (v) => ({ 'GetEvents': { 'collection': v.collection, 'event_ids': v.eventIds } }),
      Fetch: (v) => ({ 'Fetch': { 'collection': v.collection, 'selection': v.selection, 'known_matches': v.knownMatches } }),
      SubscribeQuery: (v) => ({ 'SubscribeQuery': { 'query_id': v.queryId, 'collection': v.collection, 'selection': v.selection, 'version': v.version, 'known_matches': v.knownMatches } }),
    });
  }

  static fromJson(value: unknown): Result<NodeRequestBody, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      if ('CommitTransaction' in o) {
        const p = o['CommitTransaction'];
        return Result.Ok(new NodeRequestBody('CommitTransaction', { id: ((v: unknown) => _take(TransactionId.fromJson(v)))((p as Record<string, unknown>)['id']), events: ((v: unknown) => (v as unknown[]).map((v) => _take(Attested.fromJson(v))))((p as Record<string, unknown>)['events']) }));
      }
      if ('Get' in o) {
        const p = o['Get'];
        return Result.Ok(new NodeRequestBody('Get', { collection: ((v: unknown) => _take(CollectionId.fromJson(v)))((p as Record<string, unknown>)['collection']), ids: ((v: unknown) => (v as unknown[]).map((v) => _take(EntityId.fromJson(v))))((p as Record<string, unknown>)['ids']) }));
      }
      if ('GetEvents' in o) {
        const p = o['GetEvents'];
        return Result.Ok(new NodeRequestBody('GetEvents', { collection: ((v: unknown) => _take(CollectionId.fromJson(v)))((p as Record<string, unknown>)['collection']), eventIds: ((v: unknown) => (v as unknown[]).map((v) => _take(EventId.fromJson(v))))((p as Record<string, unknown>)['event_ids']) }));
      }
      if ('Fetch' in o) {
        const p = o['Fetch'];
        return Result.Ok(new NodeRequestBody('Fetch', { collection: ((v: unknown) => _take(CollectionId.fromJson(v)))((p as Record<string, unknown>)['collection']), selection: ((v: unknown) => _take(Selection.fromJson(v)))((p as Record<string, unknown>)['selection']), knownMatches: ((v: unknown) => (v as unknown[]).map((v) => _take(KnownEntity.fromJson(v))))((p as Record<string, unknown>)['known_matches']) }));
      }
      if ('SubscribeQuery' in o) {
        const p = o['SubscribeQuery'];
        return Result.Ok(new NodeRequestBody('SubscribeQuery', { queryId: ((v: unknown) => _take(QueryId.fromJson(v)))((p as Record<string, unknown>)['query_id']), collection: ((v: unknown) => _take(CollectionId.fromJson(v)))((p as Record<string, unknown>)['collection']), selection: ((v: unknown) => _take(Selection.fromJson(v)))((p as Record<string, unknown>)['selection']), version: ((v: unknown) => v as number)((p as Record<string, unknown>)['version']), knownMatches: ((v: unknown) => (v as unknown[]).map((v) => _take(KnownEntity.fromJson(v))))((p as Record<string, unknown>)['known_matches']) }));
      }
      return Result.Err(JsonError.custom('no variant of `NodeRequestBody` matches this JSON'));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
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

  toJSON(): unknown {
    return this.match<unknown>({
      CommitComplete: (v) => ({ 'CommitComplete': { 'id': v.id } }),
      Fetch: (v) => ({ 'Fetch': v._0 }),
      Get: (v) => ({ 'Get': v._0 }),
      GetEvents: (v) => ({ 'GetEvents': v._0 }),
      QuerySubscribed: (v) => ({ 'QuerySubscribed': { 'query_id': v.queryId, 'deltas': v.deltas } }),
      Success: () => 'Success',
      Error: (v) => ({ 'Error': v._0 }),
    });
  }

  static fromJson(value: unknown): Result<NodeResponseBody, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      if (typeof value === 'string') {
        switch (value) {
          case 'Success': return Result.Ok(new NodeResponseBody('Success', {}));
        }
      }
      const o = value as Record<string, unknown>;
      if ('CommitComplete' in o) {
        const p = o['CommitComplete'];
        return Result.Ok(new NodeResponseBody('CommitComplete', { id: ((v: unknown) => _take(TransactionId.fromJson(v)))((p as Record<string, unknown>)['id']) }));
      }
      if ('Fetch' in o) {
        const p = o['Fetch'];
        return Result.Ok(new NodeResponseBody('Fetch', { _0: ((v: unknown) => (v as unknown[]).map((v) => _take(EntityDelta.fromJson(v))))(p) }));
      }
      if ('Get' in o) {
        const p = o['Get'];
        return Result.Ok(new NodeResponseBody('Get', { _0: ((v: unknown) => (v as unknown[]).map((v) => _take(Attested.fromJson(v))))(p) }));
      }
      if ('GetEvents' in o) {
        const p = o['GetEvents'];
        return Result.Ok(new NodeResponseBody('GetEvents', { _0: ((v: unknown) => (v as unknown[]).map((v) => _take(Attested.fromJson(v))))(p) }));
      }
      if ('QuerySubscribed' in o) {
        const p = o['QuerySubscribed'];
        return Result.Ok(new NodeResponseBody('QuerySubscribed', { queryId: ((v: unknown) => _take(QueryId.fromJson(v)))((p as Record<string, unknown>)['query_id']), deltas: ((v: unknown) => (v as unknown[]).map((v) => _take(EntityDelta.fromJson(v))))((p as Record<string, unknown>)['deltas']) }));
      }
      if ('Error' in o) {
        const p = o['Error'];
        return Result.Ok(new NodeResponseBody('Error', { _0: ((v: unknown) => v as string)(p) }));
      }
      return Result.Err(JsonError.custom('no variant of `NodeResponseBody` matches this JSON'));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
  }
}

