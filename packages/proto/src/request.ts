// MIRRORS: ankurah/proto/src/request.rs
import { Struct, Enum, Result, JsonError, jsonAll, dropOwned, OwnershipFatal, UnsupportedShape, debugString } from '@ankurah/base';
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
    return `NodeRequest { id: ${this.id.debug()}, to: ${this.to.debug()}, from: ${this.from.debug()}, body: ${this.body.debug()} }`;
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
    return `KnownEntity { entityId: ${this.entityId.debug()}, head: ${this.head.debug()} }`;
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
    return { 'entity_id': this.entityId.toJSON(), 'head': this.head.toJSON() };
  }

  static fromJson(value: unknown): Result<KnownEntity, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `KnownEntity`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('entity_id' in _o)) {
        return Result.Err(JsonError.custom('missing field `entity_id`'));
      }
      const _rentityId = ((v: unknown) => EntityId.fromJson(v))(_o['entity_id']);
      if (_rentityId.isErr()) return Result.Err(_rentityId.unwrapErr());
      const entityId = _rentityId.unwrap();
      $built.push(entityId);
      if (!('head' in _o)) {
        return Result.Err(JsonError.custom('missing field `head`'));
      }
      const _rhead = ((v: unknown) => Clock.fromJson(v))(_o['head']);
      if (_rhead.isErr()) return Result.Err(_rhead.unwrapErr());
      const head = _rhead.unwrap();
      $built.push(head);
      const $out = new KnownEntity(entityId, head);
      $kept = true;
      return Result.Ok($out);
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
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
    return `CausalAssertion { entityId: ${this.entityId.debug()}, subject: ${this.subject.debug()}, other: ${this.other.debug()}, relation: ${this.relation.debug()} }`;
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
    return { 'entity_id': this.entityId.toJSON(), 'subject': this.subject.toJSON(), 'other': this.other.toJSON(), 'relation': this.relation.toJSON() };
  }

  static fromJson(value: unknown): Result<CausalAssertion, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `CausalAssertion`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('entity_id' in _o)) {
        return Result.Err(JsonError.custom('missing field `entity_id`'));
      }
      const _rentityId = ((v: unknown) => EntityId.fromJson(v))(_o['entity_id']);
      if (_rentityId.isErr()) return Result.Err(_rentityId.unwrapErr());
      const entityId = _rentityId.unwrap();
      $built.push(entityId);
      if (!('subject' in _o)) {
        return Result.Err(JsonError.custom('missing field `subject`'));
      }
      const _rsubject = ((v: unknown) => Clock.fromJson(v))(_o['subject']);
      if (_rsubject.isErr()) return Result.Err(_rsubject.unwrapErr());
      const subject = _rsubject.unwrap();
      $built.push(subject);
      if (!('other' in _o)) {
        return Result.Err(JsonError.custom('missing field `other`'));
      }
      const _rother = ((v: unknown) => Clock.fromJson(v))(_o['other']);
      if (_rother.isErr()) return Result.Err(_rother.unwrapErr());
      const other = _rother.unwrap();
      $built.push(other);
      if (!('relation' in _o)) {
        return Result.Err(JsonError.custom('missing field `relation`'));
      }
      const _rrelation = ((v: unknown) => CausalRelation.fromJson(v))(_o['relation']);
      if (_rrelation.isErr()) return Result.Err(_rrelation.unwrapErr());
      const relation = _rrelation.unwrap();
      $built.push(relation);
      const $out = new CausalAssertion(entityId, subject, other, relation);
      $kept = true;
      return Result.Ok($out);
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
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
    return { 'relation': this.relation.toJSON(), 'attestations': this.attestations.toJSON() };
  }

  static fromJson(value: unknown): Result<CausalAssertionFragment, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `CausalAssertionFragment`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('relation' in _o)) {
        return Result.Err(JsonError.custom('missing field `relation`'));
      }
      const _rrelation = ((v: unknown) => CausalRelation.fromJson(v))(_o['relation']);
      if (_rrelation.isErr()) return Result.Err(_rrelation.unwrapErr());
      const relation = _rrelation.unwrap();
      $built.push(relation);
      if (!('attestations' in _o)) {
        return Result.Err(JsonError.custom('missing field `attestations`'));
      }
      const _rattestations = ((v: unknown) => AttestationSet.fromJson(v))(_o['attestations']);
      if (_rattestations.isErr()) return Result.Err(_rattestations.unwrapErr());
      const attestations = _rattestations.unwrap();
      $built.push(attestations);
      const $out = new CausalAssertionFragment(relation, attestations);
      $kept = true;
      return Result.Ok($out);
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
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
    return `EntityDelta { entityId: ${this.entityId.debug()}, collection: ${this.collection.debug()}, content: ${this.content.debug()} }`;
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
    return { 'entity_id': this.entityId.toJSON(), 'collection': this.collection.toJSON(), 'content': this.content.toJSON() };
  }

  static fromJson(value: unknown): Result<EntityDelta, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `EntityDelta`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('entity_id' in _o)) {
        return Result.Err(JsonError.custom('missing field `entity_id`'));
      }
      const _rentityId = ((v: unknown) => EntityId.fromJson(v))(_o['entity_id']);
      if (_rentityId.isErr()) return Result.Err(_rentityId.unwrapErr());
      const entityId = _rentityId.unwrap();
      $built.push(entityId);
      if (!('collection' in _o)) {
        return Result.Err(JsonError.custom('missing field `collection`'));
      }
      const _rcollection = ((v: unknown) => CollectionId.fromJson(v))(_o['collection']);
      if (_rcollection.isErr()) return Result.Err(_rcollection.unwrapErr());
      const collection = _rcollection.unwrap();
      $built.push(collection);
      if (!('content' in _o)) {
        return Result.Err(JsonError.custom('missing field `content`'));
      }
      const _rcontent = ((v: unknown) => DeltaContent.fromJson(v))(_o['content']);
      if (_rcontent.isErr()) return Result.Err(_rcontent.unwrapErr());
      const content = _rcontent.unwrap();
      $built.push(content);
      const $out = new EntityDelta(entityId, collection, content);
      $kept = true;
      return Result.Ok($out);
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
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
    return `NodeResponse { requestId: ${this.requestId.debug()}, from: ${this.from.debug()}, to: ${this.to.debug()}, body: ${this.body.debug()} }`;
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
      DivergedSince: (v) => `DivergedSince { meet: ${v.meet.debug()}, subject: ${v.subject.debug()}, other: ${v.other.debug()} }`,
      Disjoint: (v) => `Disjoint { gca: ${(($v) => $v === null ? 'None' : `Some(${$v.debug()})`)(v.gca)}, subjectRoot: ${v.subjectRoot.debug()}, otherRoot: ${v.otherRoot.debug()} }`,
      BudgetExceeded: (v) => `BudgetExceeded { subject: ${v.subject.debug()}, other: ${v.other.debug()} }`,
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
      DivergedSince: (v) => ({ 'DivergedSince': { 'meet': v.meet.toJSON(), 'subject': v.subject.toJSON(), 'other': v.other.toJSON() } }),
      Disjoint: (v) => ({ 'Disjoint': { 'gca': (v.gca == null ? null : v.gca.toJSON()), 'subject_root': v.subjectRoot.toJSON(), 'other_root': v.otherRoot.toJSON() } }),
      BudgetExceeded: (v) => ({ 'BudgetExceeded': { 'subject': v.subject.toJSON(), 'other': v.other.toJSON() } }),
    });
  }

  static fromJson(value: unknown): Result<CausalRelation, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'Equal': return Result.Ok(new CausalRelation('Equal', {}));
          case 'StrictDescends': return Result.Ok(new CausalRelation('StrictDescends', {}));
          case 'StrictAscends': return Result.Ok(new CausalRelation('StrictAscends', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `CausalRelation`'));
      }
      const o = value as Record<string, unknown>;
      if ('DivergedSince' in o) {
        if (o['DivergedSince'] === null || typeof o['DivergedSince'] !== 'object' || Array.isArray(o['DivergedSince'])) {
          return Result.Err(JsonError.custom('expected an object for `CausalRelation`'));
        }
        const _o = o['DivergedSince'] as Record<string, unknown>;
        if (!('meet' in _o)) {
          return Result.Err(JsonError.custom('missing field `meet`'));
        }
        const _rmeet = ((v: unknown) => Clock.fromJson(v))(_o['meet']);
        if (_rmeet.isErr()) return Result.Err(_rmeet.unwrapErr());
        const meet = _rmeet.unwrap();
        $built.push(meet);
        if (!('subject' in _o)) {
          return Result.Err(JsonError.custom('missing field `subject`'));
        }
        const _rsubject = ((v: unknown) => Clock.fromJson(v))(_o['subject']);
        if (_rsubject.isErr()) return Result.Err(_rsubject.unwrapErr());
        const subject = _rsubject.unwrap();
        $built.push(subject);
        if (!('other' in _o)) {
          return Result.Err(JsonError.custom('missing field `other`'));
        }
        const _rother = ((v: unknown) => Clock.fromJson(v))(_o['other']);
        if (_rother.isErr()) return Result.Err(_rother.unwrapErr());
        const other = _rother.unwrap();
        $built.push(other);
        
        const $out = new CausalRelation('DivergedSince', { meet: meet, subject: subject, other: other });
        $kept = true;
        return Result.Ok($out);
      }
      if ('Disjoint' in o) {
        if (o['Disjoint'] === null || typeof o['Disjoint'] !== 'object' || Array.isArray(o['Disjoint'])) {
          return Result.Err(JsonError.custom('expected an object for `CausalRelation`'));
        }
        const _o = o['Disjoint'] as Record<string, unknown>;
        const _rgca = ((v: unknown) => (v == null ? Result.Ok(null) : ((v: unknown) => Clock.fromJson(v))(v)))(_o['gca']);
        if (_rgca.isErr()) return Result.Err(_rgca.unwrapErr());
        const gca = _rgca.unwrap();
        $built.push(gca);
        if (!('subject_root' in _o)) {
          return Result.Err(JsonError.custom('missing field `subject_root`'));
        }
        const _rsubjectRoot = ((v: unknown) => EventId.fromJson(v))(_o['subject_root']);
        if (_rsubjectRoot.isErr()) return Result.Err(_rsubjectRoot.unwrapErr());
        const subjectRoot = _rsubjectRoot.unwrap();
        $built.push(subjectRoot);
        if (!('other_root' in _o)) {
          return Result.Err(JsonError.custom('missing field `other_root`'));
        }
        const _rotherRoot = ((v: unknown) => EventId.fromJson(v))(_o['other_root']);
        if (_rotherRoot.isErr()) return Result.Err(_rotherRoot.unwrapErr());
        const otherRoot = _rotherRoot.unwrap();
        $built.push(otherRoot);
        
        const $out = new CausalRelation('Disjoint', { gca: gca, subjectRoot: subjectRoot, otherRoot: otherRoot });
        $kept = true;
        return Result.Ok($out);
      }
      if ('BudgetExceeded' in o) {
        if (o['BudgetExceeded'] === null || typeof o['BudgetExceeded'] !== 'object' || Array.isArray(o['BudgetExceeded'])) {
          return Result.Err(JsonError.custom('expected an object for `CausalRelation`'));
        }
        const _o = o['BudgetExceeded'] as Record<string, unknown>;
        if (!('subject' in _o)) {
          return Result.Err(JsonError.custom('missing field `subject`'));
        }
        const _rsubject = ((v: unknown) => Clock.fromJson(v))(_o['subject']);
        if (_rsubject.isErr()) return Result.Err(_rsubject.unwrapErr());
        const subject = _rsubject.unwrap();
        $built.push(subject);
        if (!('other' in _o)) {
          return Result.Err(JsonError.custom('missing field `other`'));
        }
        const _rother = ((v: unknown) => Clock.fromJson(v))(_o['other']);
        if (_rother.isErr()) return Result.Err(_rother.unwrapErr());
        const other = _rother.unwrap();
        $built.push(other);
        
        const $out = new CausalRelation('BudgetExceeded', { subject: subject, other: other });
        $kept = true;
        return Result.Ok($out);
      }
      return Result.Err(JsonError.custom('no variant of `CausalRelation` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
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
      StateSnapshot: (v) => ({ 'StateSnapshot': { 'state': v.state.toJSON() } }),
      EventBridge: (v) => ({ 'EventBridge': { 'events': v.events.map((x) => x.toJSON()) } }),
      StateAndRelation: (v) => ({ 'StateAndRelation': { 'state': v.state.toJSON(), 'relation': v.relation.toJSON() } }),
    });
  }

  static fromJson(value: unknown): Result<DeltaContent, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `DeltaContent`'));
      }
      const o = value as Record<string, unknown>;
      if ('StateSnapshot' in o) {
        if (o['StateSnapshot'] === null || typeof o['StateSnapshot'] !== 'object' || Array.isArray(o['StateSnapshot'])) {
          return Result.Err(JsonError.custom('expected an object for `DeltaContent`'));
        }
        const _o = o['StateSnapshot'] as Record<string, unknown>;
        if (!('state' in _o)) {
          return Result.Err(JsonError.custom('missing field `state`'));
        }
        const _rstate = ((v: unknown) => StateFragment.fromJson(v))(_o['state']);
        if (_rstate.isErr()) return Result.Err(_rstate.unwrapErr());
        const state = _rstate.unwrap();
        $built.push(state);
        
        const $out = new DeltaContent('StateSnapshot', { state: state });
        $kept = true;
        return Result.Ok($out);
      }
      if ('EventBridge' in o) {
        if (o['EventBridge'] === null || typeof o['EventBridge'] !== 'object' || Array.isArray(o['EventBridge'])) {
          return Result.Err(JsonError.custom('expected an object for `DeltaContent`'));
        }
        const _o = o['EventBridge'] as Record<string, unknown>;
        if (!('events' in _o)) {
          return Result.Err(JsonError.custom('missing field `events`'));
        }
        const _revents = ((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => EventFragment.fromJson(v))) : Result.Err(JsonError.custom('expected an array'))))(_o['events']);
        if (_revents.isErr()) return Result.Err(_revents.unwrapErr());
        const events = _revents.unwrap();
        $built.push(events);
        
        const $out = new DeltaContent('EventBridge', { events: events });
        $kept = true;
        return Result.Ok($out);
      }
      if ('StateAndRelation' in o) {
        if (o['StateAndRelation'] === null || typeof o['StateAndRelation'] !== 'object' || Array.isArray(o['StateAndRelation'])) {
          return Result.Err(JsonError.custom('expected an object for `DeltaContent`'));
        }
        const _o = o['StateAndRelation'] as Record<string, unknown>;
        if (!('state' in _o)) {
          return Result.Err(JsonError.custom('missing field `state`'));
        }
        const _rstate = ((v: unknown) => StateFragment.fromJson(v))(_o['state']);
        if (_rstate.isErr()) return Result.Err(_rstate.unwrapErr());
        const state = _rstate.unwrap();
        $built.push(state);
        if (!('relation' in _o)) {
          return Result.Err(JsonError.custom('missing field `relation`'));
        }
        const _rrelation = ((v: unknown) => CausalAssertionFragment.fromJson(v))(_o['relation']);
        if (_rrelation.isErr()) return Result.Err(_rrelation.unwrapErr());
        const relation = _rrelation.unwrap();
        $built.push(relation);
        
        const $out = new DeltaContent('StateAndRelation', { state: state, relation: relation });
        $kept = true;
        return Result.Ok($out);
      }
      return Result.Err(JsonError.custom('no variant of `DeltaContent` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
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
      CommitTransaction: (v) => `CommitTransaction { id: ${v.id.debug()}, events: ${`[${Array.from(v.events).map((e) => e.debug()).join(', ')}]`} }`,
      Get: (v) => `Get { collection: ${v.collection.debug()}, ids: ${`[${Array.from(v.ids).map((e) => e.debug()).join(', ')}]`} }`,
      GetEvents: (v) => `GetEvents { collection: ${v.collection.debug()}, eventIds: ${`[${Array.from(v.eventIds).map((e) => e.debug()).join(', ')}]`} }`,
      Fetch: (v) => `Fetch { collection: ${v.collection.debug()}, selection: ${v.selection.debug()}, knownMatches: ${`[${Array.from(v.knownMatches).map((e) => e.debug()).join(', ')}]`} }`,
      SubscribeQuery: (v) => `SubscribeQuery { queryId: ${v.queryId.debug()}, collection: ${v.collection.debug()}, selection: ${v.selection.debug()}, version: ${String(v.version)}, knownMatches: ${`[${Array.from(v.knownMatches).map((e) => e.debug()).join(', ')}]`} }`,
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
      CommitComplete: (v) => `CommitComplete { id: ${v.id.debug()} }`,
      Fetch: (v) => `Fetch(${`[${Array.from(v._0).map((e) => e.debug()).join(', ')}]`})`,
      Get: (v) => `Get(${`[${Array.from(v._0).map((e) => e.debug()).join(', ')}]`})`,
      GetEvents: (v) => `GetEvents(${`[${Array.from(v._0).map((e) => e.debug()).join(', ')}]`})`,
      QuerySubscribed: (v) => `QuerySubscribed { queryId: ${v.queryId.debug()}, deltas: ${`[${Array.from(v.deltas).map((e) => e.debug()).join(', ')}]`} }`,
      Success: () => 'Success',
      Error: (v) => `Error(${debugString(v._0)})`,
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

