// MIRRORS: ankurah/core/src/error.rs
import { Struct, Enum, AnyhowError, anyhow, checkedAdd, HashSet, JoinError } from '@ankurah/base';
import { CollectionId, DecodeError, EntityId, EventId } from '@ankurah/proto';
import { SendError } from './connector';
import { AccessDenied } from './policy';
import { PropertyError } from './property/traits';
import { Subscription } from './reactor/subscription_state';
import { Error } from './selection/filter';
import { ParseError } from '@ankurah/ankql';
import { CollectionId, DecodeError, EntityId, EventId, NodeResponseBody } from '@ankurah/proto';

export class ApplyErrorItem extends Struct {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly cause: MutationError;

  constructor(entityId: EntityId, collection: CollectionId, cause: MutationError) {
    super();
    this.entityId = entityId;
    this.collection = collection;
    this.cause = cause;
  }

  toString(): string {
    return `Failed to apply delta for entity ${this.entityId.toBase64Short()} in collection ${this.collection}: ${this.cause}`;
  }

  debug(): string {
    return `ApplyErrorItem { entityId: ${this.entityId.debug()}, collection: ${this.collection.debug()}, cause: ${this.cause.debug()} }`;
  }
}

export type RetrievalErrorV = {
  AccessDenied: { _0: AccessDenied };
  ParseError: { _0: ParseError };
  EntityNotFound: { _0: EntityId };
  EventNotFound: { _0: EventId };
  StorageError: { _0: Error };
  CollectionNotFound: { _0: CollectionId };
  FailedUpdate: { _0: Error };
  DeserializationError: { _0: Error };
  NoDurablePeers: {};
  Other: { _0: string };
  InvalidBucketName: {};
  AnkqlFilter: { _0: Error };
  FutureJoin: { _0: JoinError };
  Anyhow: { _0: Error };
  DecodeError: { _0: DecodeError };
  StateError: { _0: StateError };
  MutationError: { _0: MutationError };
  PropertyError: { _0: PropertyError };
  RequestError: { _0: RequestError };
  ApplyError: { _0: ApplyError };
};

export class RetrievalError extends Enum<RetrievalErrorV> {

  static storage(err: Error): RetrievalError {
    return new RetrievalError('StorageError', { _0: err });
  }

  static fromRequestError(err: RequestError): RetrievalError {
    return new RetrievalError('RequestError', { _0: err });
  }

  static fromPropertyError(err: PropertyError): RetrievalError {
    return new RetrievalError('PropertyError', { _0: err });
  }

  static fromJoinError(err: JoinError): RetrievalError {
    return new RetrievalError('FutureJoin', { _0: err });
  }

  static fromMutationError(err: MutationError): RetrievalError {
    return new RetrievalError('MutationError', { _0: err });
  }

  static fromError(e: Error): RetrievalError {
    return new RetrievalError('DeserializationError', { _0: e });
  }

  static fromDecodeError(err: DecodeError): RetrievalError {
    return new RetrievalError('DecodeError', { _0: err });
  }

  static fromAccessDenied(err: AccessDenied): RetrievalError {
    return new RetrievalError('AccessDenied', { _0: err });
  }

  static fromSubscriptionError(err: SubscriptionError): RetrievalError {
    try {
      return new RetrievalError('Anyhow', { _0: AnyhowError.msg(`Subscription error: ${err.debug()}`) });
    } finally {
      err.drop();
    }
  }

  static fromStateError(err: StateError): RetrievalError {
    return new RetrievalError('StateError', { _0: err });
  }

  static fromApplyError(err: ApplyError): RetrievalError {
    return new RetrievalError('ApplyError', { _0: err });
  }

  debug(): string {
    return this.match({
      AccessDenied: (v) => `AccessDenied(${v._0.debug()})`,
      ParseError: (v) => `ParseError(${v._0.debug()})`,
      EntityNotFound: (v) => `EntityNotFound(${v._0.debug()})`,
      EventNotFound: (v) => `EventNotFound(${v._0.debug()})`,
      StorageError: (v) => `StorageError(${v._0})`,
      CollectionNotFound: (v) => `CollectionNotFound(${v._0.debug()})`,
      FailedUpdate: (v) => `FailedUpdate(${v._0})`,
      DeserializationError: (v) => `DeserializationError(${v._0})`,
      NoDurablePeers: () => 'NoDurablePeers',
      Other: (v) => `Other(${JSON.stringify(v._0)})`,
      InvalidBucketName: () => 'InvalidBucketName',
      AnkqlFilter: (v) => `AnkqlFilter(${v._0.debug()})`,
      FutureJoin: (v) => `FutureJoin(${v._0})`,
      Anyhow: (v) => `Anyhow(${v._0})`,
      DecodeError: (v) => `DecodeError(${v._0.debug()})`,
      StateError: (v) => `StateError(${v._0.debug()})`,
      MutationError: (v) => `MutationError(${v._0.debug()})`,
      PropertyError: (v) => `PropertyError(${v._0.debug()})`,
      RequestError: (v) => `RequestError(${v._0.debug()})`,
      ApplyError: (v) => `ApplyError(${v._0.debug()})`,
    });
  }

  override toString(): string {
    return this.match({
      AccessDenied: () => 'access denied',
      ParseError: (v) => `Parse error: ${v._0}`,
      EntityNotFound: (v) => `Entity not found: ${v._0.debug()}`,
      EventNotFound: (v) => `Event not found: ${v._0.debug()}`,
      StorageError: (v) => `Storage error: ${v._0}`,
      CollectionNotFound: (v) => `Collection not found: ${v._0}`,
      FailedUpdate: (v) => `Update failed: ${v._0}`,
      DeserializationError: (v) => `Deserialization error: ${v._0}`,
      NoDurablePeers: () => 'No durable peers available for fetch operation',
      Other: (v) => `Other error: ${v._0}`,
      InvalidBucketName: () => 'bucket name must only contain valid characters',
      AnkqlFilter: (v) => `ankql filter: ${v._0}`,
      FutureJoin: (v) => `Future join: ${v._0}`,
      Anyhow: (v) => `${v._0}`,
      DecodeError: (v) => `Decode error: ${v._0}`,
      StateError: (v) => `State error: ${v._0}`,
      MutationError: (v) => `Mutation error: ${v._0}`,
      PropertyError: (v) => `Property error: ${v._0}`,
      RequestError: (v) => `Request error: ${v._0}`,
      ApplyError: (v) => `Apply error: ${v._0}`,
    });
  }
}

export type RequestErrorV = {
  PeerNotConnected: {};
  ConnectionLost: {};
  ServerError: { _0: string };
  SendError: { _0: SendError };
  InternalChannelClosed: {};
  UnexpectedResponse: { _0: NodeResponseBody };
  AccessDenied: { _0: AccessDenied };
};

export class RequestError extends Enum<RequestErrorV> {

  static fromAccessDenied(err: AccessDenied): RequestError {
    return new RequestError('AccessDenied', { _0: err });
  }

  static fromSendError(err: SendError): RequestError {
    return new RequestError('SendError', { _0: err });
  }

  debug(): string {
    return this.match({
      PeerNotConnected: () => 'PeerNotConnected',
      ConnectionLost: () => 'ConnectionLost',
      ServerError: (v) => `ServerError(${JSON.stringify(v._0)})`,
      SendError: (v) => `SendError(${v._0.debug()})`,
      InternalChannelClosed: () => 'InternalChannelClosed',
      UnexpectedResponse: (v) => `UnexpectedResponse(${v._0.debug()})`,
      AccessDenied: (v) => `AccessDenied(${v._0.debug()})`,
    });
  }

  override toString(): string {
    return this.match({
      PeerNotConnected: () => 'Peer not connected',
      ConnectionLost: () => 'Connection lost',
      ServerError: (v) => `Server error: ${v._0}`,
      SendError: (v) => `Send error: ${v._0}`,
      InternalChannelClosed: () => 'Internal channel closed',
      UnexpectedResponse: (v) => `Unexpected response: ${v._0.debug()}`,
      AccessDenied: (v) => `Access denied: ${v._0}`,
    });
  }
}

export type SubscriptionErrorV = {
  PredicateNotFound: {};
  PredicateAlreadySubscribed: {};
  SubscriptionNotFound: {};
};

export class SubscriptionError extends Enum<SubscriptionErrorV> {

  debug(): string {
    return this.match({
      PredicateNotFound: () => 'PredicateNotFound',
      PredicateAlreadySubscribed: () => 'PredicateAlreadySubscribed',
      SubscriptionNotFound: () => 'SubscriptionNotFound',
    });
  }

  override toString(): string {
    return this.match({
      PredicateNotFound: () => 'predicate not found',
      PredicateAlreadySubscribed: () => 'already subscribed to predicate',
      SubscriptionNotFound: () => 'subscription not found',
    });
  }
}

export type MutationErrorV = {
  AccessDenied: { _0: AccessDenied };
  AlreadyExists: {};
  RetrievalError: { _0: RetrievalError };
  StateError: { _0: StateError };
  UpdateFailed: { _0: Error };
  FailedStep: { _0: string; _1: string };
  FailedToSetProperty: { _0: string; _1: string };
  General: { _0: Error };
  NoDurablePeers: {};
  DecodeError: { _0: DecodeError };
  LineageError: { _0: LineageError };
  PeerRejected: {};
  InvalidEvent: {};
  InvalidUpdate: { _0: string };
  PropertyError: { _0: PropertyError };
  FutureJoin: { _0: JoinError };
  Anyhow: { _0: Error };
  TOCTOUAttemptsExhausted: {};
};

export class MutationError extends Enum<MutationErrorV> {

  static fromJoinError(err: JoinError): MutationError {
    return new MutationError('FutureJoin', { _0: err });
  }

  static fromError(err: AnyhowError): MutationError {
    return new MutationError('Anyhow', { _0: err });
  }

  static fromLineageError(err: LineageError): MutationError {
    return new MutationError('LineageError', { _0: err });
  }

  static fromDecodeError(err: DecodeError): MutationError {
    return new MutationError('DecodeError', { _0: err });
  }

  static fromAccessDenied(err: AccessDenied): MutationError {
    return new MutationError('AccessDenied', { _0: err });
  }

  static fromRetrievalError(err: RetrievalError): MutationError {
    return err.intoMatch({
      AccessDenied: (v) => {
        const a = v._0;
        return new MutationError('AccessDenied', { _0: a });
      },
      ParseError: (v) => {
        const err = new RetrievalError('ParseError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      EntityNotFound: (v) => {
        const err = new RetrievalError('EntityNotFound', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      EventNotFound: (v) => {
        const err = new RetrievalError('EventNotFound', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      StorageError: (v) => {
        const err = new RetrievalError('StorageError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      CollectionNotFound: (v) => {
        const err = new RetrievalError('CollectionNotFound', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      FailedUpdate: (v) => {
        const err = new RetrievalError('FailedUpdate', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      DeserializationError: (v) => {
        const err = new RetrievalError('DeserializationError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      NoDurablePeers: (v) => {
        const err = new RetrievalError('NoDurablePeers', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      Other: (v) => {
        const err = new RetrievalError('Other', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      InvalidBucketName: (v) => {
        const err = new RetrievalError('InvalidBucketName', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      AnkqlFilter: (v) => {
        const err = new RetrievalError('AnkqlFilter', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      FutureJoin: (v) => {
        const err = new RetrievalError('FutureJoin', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      Anyhow: (v) => {
        const err = new RetrievalError('Anyhow', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      DecodeError: (v) => {
        const err = new RetrievalError('DecodeError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      StateError: (v) => {
        const err = new RetrievalError('StateError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      MutationError: (v) => {
        const err = new RetrievalError('MutationError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      PropertyError: (v) => {
        const err = new RetrievalError('PropertyError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      RequestError: (v) => {
        const err = new RetrievalError('RequestError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
      ApplyError: (v) => {
        const err = new RetrievalError('ApplyError', v);
        return new MutationError('RetrievalError', { _0: err });
      },
    });
  }

  static fromStateError(err: StateError): MutationError {
    return new MutationError('StateError', { _0: err });
  }

  static fromPropertyError(err: PropertyError): MutationError {
    return new MutationError('PropertyError', { _0: err });
  }

  debug(): string {
    return this.match({
      AccessDenied: (v) => `AccessDenied(${v._0.debug()})`,
      AlreadyExists: () => 'AlreadyExists',
      RetrievalError: (v) => `RetrievalError(${v._0.debug()})`,
      StateError: (v) => `StateError(${v._0.debug()})`,
      UpdateFailed: (v) => `UpdateFailed(${v._0})`,
      FailedStep: (v) => `FailedStep(${JSON.stringify(v._0)}, ${JSON.stringify(v._1)})`,
      FailedToSetProperty: (v) => `FailedToSetProperty(${JSON.stringify(v._0)}, ${JSON.stringify(v._1)})`,
      General: (v) => `General(${v._0})`,
      NoDurablePeers: () => 'NoDurablePeers',
      DecodeError: (v) => `DecodeError(${v._0.debug()})`,
      LineageError: (v) => `LineageError(${v._0.debug()})`,
      PeerRejected: () => 'PeerRejected',
      InvalidEvent: () => 'InvalidEvent',
      InvalidUpdate: (v) => `InvalidUpdate(${JSON.stringify(v._0)})`,
      PropertyError: (v) => `PropertyError(${v._0.debug()})`,
      FutureJoin: (v) => `FutureJoin(${v._0})`,
      Anyhow: (v) => `Anyhow(${v._0})`,
      TOCTOUAttemptsExhausted: () => 'TOCTOUAttemptsExhausted',
    });
  }

  override toString(): string {
    return this.match({
      AccessDenied: () => 'access denied',
      AlreadyExists: () => 'already exists',
      RetrievalError: (v) => `retrieval error: ${v._0}`,
      StateError: (v) => `state error: ${v._0}`,
      UpdateFailed: (v) => `failed update: ${v._0}`,
      FailedStep: (v) => `failed step: ${v._0}: ${v._1}`,
      FailedToSetProperty: (v) => `failed to set property: ${v._0}: ${v._1}`,
      General: (v) => `general error: ${v._0}`,
      NoDurablePeers: () => 'no durable peers available',
      DecodeError: (v) => `decode error: ${v._0}`,
      LineageError: (v) => `lineage error: ${v._0}`,
      PeerRejected: () => 'peer rejected transaction',
      InvalidEvent: () => 'invalid event',
      InvalidUpdate: () => 'invalid update',
      PropertyError: (v) => `property error: ${v._0}`,
      FutureJoin: (v) => `future join: ${v._0}`,
      Anyhow: (v) => `anyhow error: ${v._0}`,
      TOCTOUAttemptsExhausted: () => 'TOCTOU attempts exhausted',
    });
  }
}

export type LineageErrorV = {
  Incomparable: {};
  PartiallyDescends: { meet: EventId[] };
  BudgetExceeded: { originalBudget: number; subjectFrontier: HashSet<EventId>; otherFrontier: HashSet<EventId> };
};

export class LineageError extends Enum<LineageErrorV> {

  toString(): string {
    let _result = '';
    return this.match({
      Incomparable: () => 'incomparable',
      PartiallyDescends: (v) => {
        const meet = v.meet;
        _result += 'partially descends: [';
        const meets = [...meet].map((id) => id.toBase64Short());
        _result += `${meets.join(', ')}]`;
        return _result;
      },
      BudgetExceeded: (v) => {
        const originalBudget = v.originalBudget;
        const subjectFrontier = v.subjectFrontier;
        const otherFrontier = v.otherFrontier;
        const subject = [...subjectFrontier].map((id) => id.toBase64Short());
        const other = [...otherFrontier].map((id) => id.toBase64Short());
        _result += `budget exceeded (${originalBudget}): subject[${subject.join(', ')}] other[${other.join(', ')}]`;
        return _result;
      },
    });
  }

  debug(): string {
    return this.match({
      Incomparable: () => 'Incomparable',
      PartiallyDescends: (v) => `PartiallyDescends { meet: ${`[${Array.from(v.meet).map((e) => e.debug()).join(', ')}]`} }`,
      BudgetExceeded: (v) => `BudgetExceeded { originalBudget: ${String(v.originalBudget)}, subjectFrontier: ${v.subjectFrontier}, otherFrontier: ${v.otherFrontier} }`,
    });
  }
}

export type StateErrorV = {
  SerializationError: { _0: Error };
  DDLError: { _0: Error };
  DMLError: { _0: Error };
};

export class StateError extends Enum<StateErrorV> {

  static fromError(e: Error): StateError {
    return new StateError('SerializationError', { _0: e });
  }

  debug(): string {
    return this.match({
      SerializationError: (v) => `SerializationError(${v._0})`,
      DDLError: (v) => `DDLError(${v._0})`,
      DMLError: (v) => `DMLError(${v._0})`,
    });
  }

  override toString(): string {
    return this.match({
      SerializationError: (v) => `serialization error: ${v._0}`,
      DDLError: (v) => `DDL error: ${v._0}`,
      DMLError: (v) => `DMLError: ${v._0}`,
    });
  }
}

export type ValidationErrorV = {
  Deserialization: { _0: Error };
  ValidationFailed: { _0: string };
  Serialization: { _0: string };
  Rejected: { _0: string };
};

export class ValidationError extends Enum<ValidationErrorV> {

  debug(): string {
    return this.match({
      Deserialization: (v) => `Deserialization(${v._0})`,
      ValidationFailed: (v) => `ValidationFailed(${JSON.stringify(v._0)})`,
      Serialization: (v) => `Serialization(${JSON.stringify(v._0)})`,
      Rejected: (v) => `Rejected(${JSON.stringify(v._0)})`,
    });
  }

  override toString(): string {
    return this.match({
      Deserialization: (v) => `Deserialization error: ${v._0}`,
      ValidationFailed: (v) => `Validation failed: ${v._0}`,
      Serialization: (v) => `Serialization error: ${v._0}`,
      Rejected: (v) => `Rejected: ${v._0}`,
    });
  }
}

export type ApplyErrorV = {
  Items: { _0: ApplyErrorItem[] };
  CollectionNotFound: { _0: CollectionId };
  RetrievalError: { _0: RetrievalError };
  MutationError: { _0: MutationError };
};

export class ApplyError extends Enum<ApplyErrorV> {

  toString(): string {
    let _result = '';
    return this.match({
      Items: (v) => {
        const errors = v._0;
        _result += `Failed to apply ${errors.length} delta(s)`;
        for (const [i, err] of [...errors].entries()) {
          _result += `\n  [${checkedAdd(i, 1, 'usize')}] ${err}`;
        }
        return _result;
      },
      CollectionNotFound: (v) => {
        const id = v._0;
        return `Collection not found: ${id}`;
      },
      RetrievalError: (v) => {
        const e = v._0;
        return `Retrieval error: ${e}`;
      },
      MutationError: (v) => {
        const e = v._0;
        return `Mutation error: ${e}`;
      },
    });
  }

  source(): Error | null {
    return this.match({
      RetrievalError: (v) => {
        const e = v._0;
        return e;
      },
      MutationError: (v) => {
        const e = v._0;
        return e;
      },
      Items: () => null,
      CollectionNotFound: () => null,
    });
  }

  static fromRetrievalError(err: RetrievalError): ApplyError {
    return new ApplyError('RetrievalError', { _0: err });
  }

  static fromMutationError(err: MutationError): ApplyError {
    return new ApplyError('MutationError', { _0: err });
  }

  debug(): string {
    return this.match({
      Items: (v) => `Items(${`[${Array.from(v._0).map((e) => e.debug()).join(', ')}]`})`,
      CollectionNotFound: (v) => `CollectionNotFound(${v._0.debug()})`,
      RetrievalError: (v) => `RetrievalError(${v._0.debug()})`,
      MutationError: (v) => `MutationError(${v._0.debug()})`,
    });
  }
}

