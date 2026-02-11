// MIRRORS: ankurah/core/src/error.rs

import { CollectionId, DecodeError, EntityId, EventId, NodeResponseBody } from '@ankurah/proto';
import { ParseError } from '@ankurah/ankql';

// Forward-reference types that will be ported in their own files.
// These use string/Error placeholders to avoid circular import issues
// until the full modules are ported.

// ─── AccessDenied ───────────────────────────────────────────────────────────
// Mirrors: ankurah/core/src/policy.rs — AccessDenied enum

export type AccessDeniedKind =
  | 'ByPolicy'
  | 'CollectionDenied'
  | 'PropertyError'
  | 'ParseError'
  | 'InsufficientAttestation';

export class AccessDenied extends Error {
  readonly kind: AccessDeniedKind;
  readonly detail?: unknown;

  constructor(kind: AccessDeniedKind, message: string, detail?: unknown) {
    super(message);
    this.name = 'AccessDenied';
    this.kind = kind;
    this.detail = detail;
  }

  static byPolicy(reason: string): AccessDenied {
    return new AccessDenied('ByPolicy', `Access denied by policy: ${reason}`, reason);
  }

  static collectionDenied(collection: CollectionId): AccessDenied {
    return new AccessDenied('CollectionDenied', `Access denied by collection: ${collection}`, collection);
  }

  static propertyError(err: Error): AccessDenied {
    return new AccessDenied('PropertyError', `Access denied by property error: ${err.message}`, err);
  }

  static parseError(err: ParseError): AccessDenied {
    return new AccessDenied('ParseError', `Access denied by parse error: ${err.message}`, err);
  }

  static insufficientAttestation(): AccessDenied {
    return new AccessDenied('InsufficientAttestation', 'Insufficient attestation');
  }
}

// ─── SendError ──────────────────────────────────────────────────────────────
// Mirrors: ankurah/core/src/connector.rs — SendError enum

export type SendErrorKind =
  | 'ConnectionClosed'
  | 'Timeout'
  | 'Other'
  | 'Unknown';

export class SendError extends Error {
  readonly kind: SendErrorKind;

  constructor(kind: SendErrorKind, message?: string) {
    super(message ?? sendErrorMessage(kind));
    this.name = 'SendError';
    this.kind = kind;
  }

  static connectionClosed(): SendError {
    return new SendError('ConnectionClosed');
  }

  static timeout(): SendError {
    return new SendError('Timeout');
  }

  static other(message: string): SendError {
    return new SendError('Other', `Other error: ${message}`);
  }

  static unknown(): SendError {
    return new SendError('Unknown');
  }
}

function sendErrorMessage(kind: SendErrorKind): string {
  switch (kind) {
    case 'ConnectionClosed': return 'Connection closed';
    case 'Timeout': return 'Send timeout';
    case 'Other': return 'Other error';
    case 'Unknown': return 'Unknown error';
  }
}

// ─── FilterError ────────────────────────────────────────────────────────────
// Mirrors: ankurah/core/src/selection/filter.rs — Error enum

export type FilterErrorKind =
  | 'CollectionMismatch'
  | 'PropertyNotFound'
  | 'UnsupportedExpression'
  | 'UnsupportedOperator';

export class FilterError extends Error {
  readonly kind: FilterErrorKind;

  constructor(kind: FilterErrorKind, message: string) {
    super(message);
    this.name = 'FilterError';
    this.kind = kind;
  }

  static collectionMismatch(expected: string, actual: string): FilterError {
    return new FilterError('CollectionMismatch', `collection mismatch: expected ${expected}, got ${actual}`);
  }

  static propertyNotFound(name: string): FilterError {
    return new FilterError('PropertyNotFound', `property not found: ${name}`);
  }

  static unsupportedExpression(detail: string): FilterError {
    return new FilterError('UnsupportedExpression', `Unsupported expression: ${detail}`);
  }

  static unsupportedOperator(detail: string): FilterError {
    return new FilterError('UnsupportedOperator', `Unsupported operator: ${detail}`);
  }
}

// ─── StateError ─────────────────────────────────────────────────────────────

export type StateErrorKind =
  | 'SerializationError'
  | 'DDLError'
  | 'DMLError';

export class StateError extends Error {
  readonly kind: StateErrorKind;
  readonly cause?: Error;

  constructor(kind: StateErrorKind, message: string, cause?: Error) {
    super(message);
    this.name = 'StateError';
    this.kind = kind;
    this.cause = cause;
  }

  static serializationError(err: Error): StateError {
    return new StateError('SerializationError', `serialization error: ${err.message}`, err);
  }

  static ddlError(err: Error): StateError {
    return new StateError('DDLError', `DDL error: ${err.message}`, err);
  }

  static dmlError(err: Error): StateError {
    return new StateError('DMLError', `DMLError: ${err.message}`, err);
  }
}

// ─── SubscriptionError ──────────────────────────────────────────────────────

export type SubscriptionErrorKind =
  | 'PredicateNotFound'
  | 'PredicateAlreadySubscribed'
  | 'SubscriptionNotFound';

export class SubscriptionError extends Error {
  readonly kind: SubscriptionErrorKind;

  constructor(kind: SubscriptionErrorKind, message?: string) {
    super(message ?? subscriptionErrorMessage(kind));
    this.name = 'SubscriptionError';
    this.kind = kind;
  }

  static predicateNotFound(): SubscriptionError {
    return new SubscriptionError('PredicateNotFound');
  }

  static predicateAlreadySubscribed(): SubscriptionError {
    return new SubscriptionError('PredicateAlreadySubscribed');
  }

  static subscriptionNotFound(): SubscriptionError {
    return new SubscriptionError('SubscriptionNotFound');
  }
}

function subscriptionErrorMessage(kind: SubscriptionErrorKind): string {
  switch (kind) {
    case 'PredicateNotFound': return 'predicate not found';
    case 'PredicateAlreadySubscribed': return 'already subscribed to predicate';
    case 'SubscriptionNotFound': return 'subscription not found';
  }
}

// ─── ValidationError ────────────────────────────────────────────────────────

export type ValidationErrorKind =
  | 'Deserialization'
  | 'ValidationFailed'
  | 'Serialization'
  | 'Rejected';

export class ValidationError extends Error {
  readonly kind: ValidationErrorKind;

  constructor(kind: ValidationErrorKind, message: string) {
    super(message);
    this.name = 'ValidationError';
    this.kind = kind;
  }

  static deserialization(err: Error): ValidationError {
    return new ValidationError('Deserialization', `Deserialization error: ${err.message}`);
  }

  static validationFailed(reason: string): ValidationError {
    return new ValidationError('ValidationFailed', `Validation failed: ${reason}`);
  }

  static serialization(reason: string): ValidationError {
    return new ValidationError('Serialization', `Serialization error: ${reason}`);
  }

  static rejected(reason: string): ValidationError {
    return new ValidationError('Rejected', `Rejected: ${reason}`);
  }
}

// ─── LineageError ───────────────────────────────────────────────────────────

export type LineageErrorKind =
  | 'Incomparable'
  | 'PartiallyDescends'
  | 'BudgetExceeded';

export class LineageError extends Error {
  readonly kind: LineageErrorKind;
  /** Present when kind is 'PartiallyDescends' */
  readonly meet?: EventId[];
  /** Present when kind is 'BudgetExceeded' */
  readonly originalBudget?: number;
  readonly subjectFrontier?: Set<EventId>;
  readonly otherFrontier?: Set<EventId>;

  private constructor(
    kind: LineageErrorKind,
    message: string,
    fields?: {
      meet?: EventId[];
      originalBudget?: number;
      subjectFrontier?: Set<EventId>;
      otherFrontier?: Set<EventId>;
    },
  ) {
    super(message);
    this.name = 'LineageError';
    this.kind = kind;
    this.meet = fields?.meet;
    this.originalBudget = fields?.originalBudget;
    this.subjectFrontier = fields?.subjectFrontier;
    this.otherFrontier = fields?.otherFrontier;
  }

  static incomparable(): LineageError {
    return new LineageError('Incomparable', 'incomparable');
  }

  static partiallyDescends(meet: EventId[]): LineageError {
    const meetStrs = meet.map((id) => id.toBase64Short());
    return new LineageError(
      'PartiallyDescends',
      `partially descends: [${meetStrs.join(', ')}]`,
      { meet },
    );
  }

  static budgetExceeded(
    originalBudget: number,
    subjectFrontier: Set<EventId>,
    otherFrontier: Set<EventId>,
  ): LineageError {
    const subjectStrs = Array.from(subjectFrontier).map((id) => id.toBase64Short());
    const otherStrs = Array.from(otherFrontier).map((id) => id.toBase64Short());
    return new LineageError(
      'BudgetExceeded',
      `budget exceeded (${originalBudget}): subject[${subjectStrs.join(', ')}] other[${otherStrs.join(', ')}]`,
      { originalBudget, subjectFrontier, otherFrontier },
    );
  }
}

// ─── RequestError ───────────────────────────────────────────────────────────

export type RequestErrorKind =
  | 'PeerNotConnected'
  | 'ConnectionLost'
  | 'ServerError'
  | 'SendError'
  | 'InternalChannelClosed'
  | 'UnexpectedResponse'
  | 'AccessDenied';

export class RequestError extends Error {
  readonly kind: RequestErrorKind;
  readonly detail?: unknown;

  constructor(kind: RequestErrorKind, message: string, detail?: unknown) {
    super(message);
    this.name = 'RequestError';
    this.kind = kind;
    this.detail = detail;
  }

  static peerNotConnected(): RequestError {
    return new RequestError('PeerNotConnected', 'Peer not connected');
  }

  static connectionLost(): RequestError {
    return new RequestError('ConnectionLost', 'Connection lost');
  }

  static serverError(message: string): RequestError {
    return new RequestError('ServerError', `Server error: ${message}`, message);
  }

  static sendError(err: SendError): RequestError {
    return new RequestError('SendError', `Send error: ${err.message}`, err);
  }

  static internalChannelClosed(): RequestError {
    return new RequestError('InternalChannelClosed', 'Internal channel closed');
  }

  static unexpectedResponse(body: NodeResponseBody): RequestError {
    return new RequestError('UnexpectedResponse', `Unexpected response: ${JSON.stringify(body)}`, body);
  }

  static accessDenied(err: AccessDenied): RequestError {
    return new RequestError('AccessDenied', `Access denied: ${err.message}`, err);
  }

  static fromAccessDenied(err: AccessDenied): RequestError {
    return RequestError.accessDenied(err);
  }

  static fromSendError(err: SendError): RequestError {
    return RequestError.sendError(err);
  }
}

// ─── ApplyErrorItem ─────────────────────────────────────────────────────────

/** Error applying a specific delta */
export class ApplyErrorItem {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly cause: MutationError;

  constructor(entityId: EntityId, collection: CollectionId, cause: MutationError) {
    this.entityId = entityId;
    this.collection = collection;
    this.cause = cause;
  }

  toString(): string {
    return `Failed to apply delta for entity ${this.entityId.toBase64Short()} in collection ${this.collection}: ${this.cause.message}`;
  }
}

// ─── ApplyError ─────────────────────────────────────────────────────────────

/** Error type for NodeApplier operations (applying remote deltas) */
export type ApplyErrorKind =
  | 'Items'
  | 'CollectionNotFound'
  | 'RetrievalError'
  | 'MutationError';

export class ApplyError extends Error {
  readonly kind: ApplyErrorKind;
  /** Present when kind is 'Items' */
  readonly items?: ApplyErrorItem[];
  /** Present when kind is 'CollectionNotFound' */
  readonly collectionId?: CollectionId;
  /** Present when kind is 'RetrievalError' */
  readonly retrievalError?: RetrievalError;
  /** Present when kind is 'MutationError' */
  readonly mutationError?: MutationError;

  private constructor(kind: ApplyErrorKind, message: string, detail?: {
    items?: ApplyErrorItem[];
    collectionId?: CollectionId;
    retrievalError?: RetrievalError;
    mutationError?: MutationError;
  }) {
    super(message);
    this.name = 'ApplyError';
    this.kind = kind;
    this.items = detail?.items;
    this.collectionId = detail?.collectionId;
    this.retrievalError = detail?.retrievalError;
    this.mutationError = detail?.mutationError;
  }

  static fromItems(items: ApplyErrorItem[]): ApplyError {
    let message = `Failed to apply ${items.length} delta(s)`;
    items.forEach((item, i) => {
      message += `\n  [${i + 1}] ${item}`;
    });
    return new ApplyError('Items', message, { items });
  }

  static collectionNotFound(id: CollectionId): ApplyError {
    return new ApplyError('CollectionNotFound', `Collection not found: ${id}`, { collectionId: id });
  }

  static fromRetrievalError(err: RetrievalError): ApplyError {
    return new ApplyError('RetrievalError', `Retrieval error: ${err.message}`, { retrievalError: err });
  }

  static fromMutationError(err: MutationError): ApplyError {
    return new ApplyError('MutationError', `Mutation error: ${err.message}`, { mutationError: err });
  }
}

// ─── MutationError ──────────────────────────────────────────────────────────

export type MutationErrorKind =
  | 'AccessDenied'
  | 'AlreadyExists'
  | 'RetrievalError'
  | 'StateError'
  | 'UpdateFailed'
  | 'FailedStep'
  | 'FailedToSetProperty'
  | 'General'
  | 'NoDurablePeers'
  | 'DecodeError'
  | 'LineageError'
  | 'PeerRejected'
  | 'InvalidEvent'
  | 'InvalidUpdate'
  | 'PropertyError'
  | 'FutureJoin'
  | 'Anyhow'
  | 'TOCTOUAttemptsExhausted';

export class MutationError extends Error {
  readonly kind: MutationErrorKind;
  readonly detail?: unknown;

  constructor(kind: MutationErrorKind, message: string, detail?: unknown) {
    super(message);
    this.name = 'MutationError';
    this.kind = kind;
    this.detail = detail;
  }

  static accessDenied(err: AccessDenied): MutationError {
    return new MutationError('AccessDenied', `access denied`, err);
  }

  static alreadyExists(): MutationError {
    return new MutationError('AlreadyExists', 'already exists');
  }

  static retrievalError(err: RetrievalError): MutationError {
    return new MutationError('RetrievalError', `retrieval error: ${err.message}`, err);
  }

  static stateError(err: StateError): MutationError {
    return new MutationError('StateError', `state error: ${err.message}`, err);
  }

  static updateFailed(err: Error): MutationError {
    return new MutationError('UpdateFailed', `failed update: ${err.message}`, err);
  }

  static failedStep(step: string, detail: string): MutationError {
    return new MutationError('FailedStep', `failed step: ${step}: ${detail}`, { step, detail });
  }

  static failedToSetProperty(property: string, detail: string): MutationError {
    return new MutationError('FailedToSetProperty', `failed to set property: ${property}: ${detail}`, { property, detail });
  }

  static general(err: Error): MutationError {
    return new MutationError('General', `general error: ${err.message}`, err);
  }

  static noDurablePeers(): MutationError {
    return new MutationError('NoDurablePeers', 'no durable peers available');
  }

  static decodeError(err: DecodeError): MutationError {
    return new MutationError('DecodeError', `decode error: ${err.message}`, err);
  }

  static lineageError(err: LineageError): MutationError {
    return new MutationError('LineageError', `lineage error: ${err.message}`, err);
  }

  static peerRejected(): MutationError {
    return new MutationError('PeerRejected', 'peer rejected transaction');
  }

  static invalidEvent(): MutationError {
    return new MutationError('InvalidEvent', 'invalid event');
  }

  static invalidUpdate(reason: string): MutationError {
    return new MutationError('InvalidUpdate', `invalid update`, reason);
  }

  static propertyError(err: Error): MutationError {
    return new MutationError('PropertyError', `property error: ${err.message}`, err);
  }

  static futureJoin(err: Error): MutationError {
    return new MutationError('FutureJoin', `future join: ${err.message}`, err);
  }

  static anyhow(err: Error): MutationError {
    return new MutationError('Anyhow', `anyhow error: ${err.message}`, err);
  }

  static toctouAttemptsExhausted(): MutationError {
    return new MutationError('TOCTOUAttemptsExhausted', 'TOCTOU attempts exhausted');
  }

  /** Convert from RetrievalError, matching Rust From<RetrievalError> for MutationError */
  static fromRetrievalError(err: RetrievalError): MutationError {
    if (err.kind === 'AccessDenied' && err.detail instanceof AccessDenied) {
      return MutationError.accessDenied(err.detail);
    }
    return MutationError.retrievalError(err);
  }

  /** Convert from StateError, matching Rust From<StateError> for MutationError */
  static fromStateError(err: StateError): MutationError {
    return MutationError.stateError(err);
  }

  /** Convert from DecodeError, matching Rust From<DecodeError> for MutationError */
  static fromDecodeError(err: DecodeError): MutationError {
    return MutationError.decodeError(err);
  }

  /** Convert from LineageError, matching Rust From<LineageError> for MutationError */
  static fromLineageError(err: LineageError): MutationError {
    return MutationError.lineageError(err);
  }

  /** Convert from AccessDenied, matching Rust From<AccessDenied> for MutationError */
  static fromAccessDenied(err: AccessDenied): MutationError {
    return MutationError.accessDenied(err);
  }
}

// ─── RetrievalError ─────────────────────────────────────────────────────────

export type RetrievalErrorKind =
  | 'AccessDenied'
  | 'ParseError'
  | 'EntityNotFound'
  | 'EventNotFound'
  | 'StorageError'
  | 'CollectionNotFound'
  | 'FailedUpdate'
  | 'DeserializationError'
  | 'NoDurablePeers'
  | 'Other'
  | 'InvalidBucketName'
  | 'AnkqlFilter'
  | 'FutureJoin'
  | 'Anyhow'
  | 'DecodeError'
  | 'StateError'
  | 'MutationError'
  | 'PropertyError'
  | 'RequestError'
  | 'ApplyError';

export class RetrievalError extends Error {
  readonly kind: RetrievalErrorKind;
  readonly detail?: unknown;

  constructor(kind: RetrievalErrorKind, message: string, detail?: unknown) {
    super(message);
    this.name = 'RetrievalError';
    this.kind = kind;
    this.detail = detail;
  }

  static accessDenied(err: AccessDenied): RetrievalError {
    return new RetrievalError('AccessDenied', 'access denied', err);
  }

  static parseError(err: ParseError): RetrievalError {
    return new RetrievalError('ParseError', `Parse error: ${err.message}`, err);
  }

  static entityNotFound(id: EntityId): RetrievalError {
    return new RetrievalError('EntityNotFound', `Entity not found: ${id}`, id);
  }

  static eventNotFound(id: EventId): RetrievalError {
    return new RetrievalError('EventNotFound', `Event not found: ${id}`, id);
  }

  static storageError(err: Error): RetrievalError {
    return new RetrievalError('StorageError', `Storage error: ${err.message}`, err);
  }

  static collectionNotFound(id: CollectionId): RetrievalError {
    return new RetrievalError('CollectionNotFound', `Collection not found: ${id}`, id);
  }

  static failedUpdate(err: Error): RetrievalError {
    return new RetrievalError('FailedUpdate', `Update failed: ${err.message}`, err);
  }

  static deserializationError(err: Error): RetrievalError {
    return new RetrievalError('DeserializationError', `Deserialization error: ${err.message}`, err);
  }

  static noDurablePeers(): RetrievalError {
    return new RetrievalError('NoDurablePeers', 'No durable peers available for fetch operation');
  }

  static other(message: string): RetrievalError {
    return new RetrievalError('Other', `Other error: ${message}`, message);
  }

  static invalidBucketName(): RetrievalError {
    return new RetrievalError('InvalidBucketName', 'bucket name must only contain valid characters');
  }

  static ankqlFilter(err: FilterError): RetrievalError {
    return new RetrievalError('AnkqlFilter', `ankql filter: ${err.message}`, err);
  }

  static futureJoin(err: Error): RetrievalError {
    return new RetrievalError('FutureJoin', `Future join: ${err.message}`, err);
  }

  static anyhow(err: Error): RetrievalError {
    return new RetrievalError('Anyhow', err.message, err);
  }

  static decodeError(err: DecodeError): RetrievalError {
    return new RetrievalError('DecodeError', `Decode error: ${err.message}`, err);
  }

  static stateError(err: StateError): RetrievalError {
    return new RetrievalError('StateError', `State error: ${err.message}`, err);
  }

  static mutationError(err: MutationError): RetrievalError {
    return new RetrievalError('MutationError', `Mutation error: ${err.message}`, err);
  }

  static propertyError(err: Error): RetrievalError {
    return new RetrievalError('PropertyError', `Property error: ${err.message}`, err);
  }

  static requestError(err: RequestError): RetrievalError {
    return new RetrievalError('RequestError', `Request error: ${err.message}`, err);
  }

  static applyError(err: ApplyError): RetrievalError {
    return new RetrievalError('ApplyError', `Apply error: ${err.message}`, err);
  }

  /** Convenience factory matching Rust RetrievalError::storage() */
  static storage(err: Error): RetrievalError {
    return RetrievalError.storageError(err);
  }

  /** Convert from AccessDenied, matching Rust From<AccessDenied> for RetrievalError */
  static fromAccessDenied(err: AccessDenied): RetrievalError {
    return RetrievalError.accessDenied(err);
  }

  /** Convert from DecodeError, matching Rust From<DecodeError> for RetrievalError */
  static fromDecodeError(err: DecodeError): RetrievalError {
    return RetrievalError.decodeError(err);
  }

  /** Convert from FilterError, matching Rust From<filter::Error> for RetrievalError */
  static fromFilterError(err: FilterError): RetrievalError {
    return RetrievalError.ankqlFilter(err);
  }

  /** Convert from StateError, matching Rust From<StateError> for RetrievalError */
  static fromStateError(err: StateError): RetrievalError {
    return RetrievalError.stateError(err);
  }

  /** Convert from MutationError, matching Rust From<MutationError> for RetrievalError */
  static fromMutationError(err: MutationError): RetrievalError {
    return RetrievalError.mutationError(err);
  }

  /** Convert from RequestError, matching Rust From<RequestError> for RetrievalError */
  static fromRequestError(err: RequestError): RetrievalError {
    return RetrievalError.requestError(err);
  }

  /** Convert from SubscriptionError, matching Rust From<SubscriptionError> for RetrievalError */
  static fromSubscriptionError(err: SubscriptionError): RetrievalError {
    return RetrievalError.anyhow(new Error(`Subscription error: ${err.message}`));
  }

  /** Convert from ApplyError, matching Rust From<ApplyError> for RetrievalError */
  static fromApplyError(err: ApplyError): RetrievalError {
    return RetrievalError.applyError(err);
  }
}
